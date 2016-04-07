extern crate enforcer;
extern crate memmap;
extern crate rustc_serialize;
extern crate docopt;
extern crate glob;
extern crate scoped_pool;
#[macro_use] extern crate log;
extern crate env_logger;

use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;
use self::memmap::{Mmap, Protection};
use enforcer::check;
use enforcer::clean;
use std::fs::File;
use std::io;
use std::io::Read;
use scoped_pool::Pool;
use docopt::Docopt;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const USAGE: &'static str = "
enforcer for code rules

Usage:
  enforcer [-g GLOB...] [-c|--clean] [-n|--count] [-t|--tabs] [-j <N>|--threads=<N>]
  enforcer (-h | --help)
  enforcer (-v | --version)
  enforcer (-s | --status)

Options:
  -g GLOB           use these glob patterns (e.g. \"**/*.h\")
  -h --help         Show this screen.
  -v --version      Show version.
  -s --status       Show configuration status.
  -n --count        only count found entries
  -c --clean        clean up trailing whitespaces
  -t --tabs         leave tabs alone (without that tabs are considered wrong)
  -j --threads=<N>  number of threads [default: 4]
";
#[derive(Debug, RustcDecodable)]
struct Args {
    flag_clean: bool,
    flag_g: Vec<String>,
    flag_version: bool,
    flag_status: bool,
    flag_count: bool,
    flag_tabs: bool,
    flag_threads: usize,
}

#[allow(dead_code)]
fn main() {
    use glob::glob;
    env_logger::init().unwrap();

    let get_cfg = || -> check::EnforcerCfg {
        fn read_enforcer_config() -> std::io::Result<check::EnforcerCfg> {
            let mut cfg_file = try!(File::open(".enforcer"));
            let mut buffer = String::new();
            try!(cfg_file.read_to_string(&mut buffer));
            check::parse_config(&buffer[..])
        }
        read_enforcer_config()
            .unwrap_or(check::default_cfg())
    };

    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());
    if args.flag_version {
        println!("  Version: {}", VERSION);
        std::process::exit(0);
    }
    let enforcer_cfg = get_cfg();
    if args.flag_status {
        println!("  using this config: {:?}", enforcer_cfg);
        std::process::exit(0);
    }
    let cfg_ignores = enforcer_cfg.ignore;
    let cfg_globs = enforcer_cfg.globs;
    let pats = if args.flag_g.len() > 0 {
        args.flag_g
    } else {
        cfg_globs
    };

    let find_matches = || -> Vec<std::path::PathBuf> {
        let relevant = |pat: &str| -> Vec<std::path::PathBuf> {
            glob(&*pat) // -> Result<Paths, PatternError>
            .ok()   // -> Option<Paths>
            .expect(&format!("glob has problems with {}", pat)[..]) // -> Paths (Iterator ofer GlobResult)
            .filter_map(Result::ok) // ignore unreadable paths -> Iterator over PathBuf
            .filter(|x| !x.components()
                        .any(|y| check::is_unwanted(y, &cfg_ignores))).collect()
        };
        pats.iter().flat_map(|pat| relevant(&pat[..])).collect()
    };
    let mut checked_files: u32 = 0;
    let mut had_tabs: u32 = 0;
    let mut had_trailing_ws: u32 = 0;
    let mut had_illegals: u32 = 0;
    let clean_f = args.flag_clean;
    let count_f = args.flag_count;
    let tabs_f = args.flag_tabs;
    let thread_count = args.flag_threads;
    println!("finding matches...");
    let paths = find_matches();
    println!("found matches...");

    let (w_chan, r_chan) = sync_channel(thread_count);
    thread::spawn(move || {
        // let (w_chan, r_chan) = sync_channel::<io::Result<u8>>(4);
        let pool = Pool::new(thread_count);

        println!("starting with {} threads....", thread_count);
        pool.scoped(|scope| {

            for path in paths {
                if !check::is_dir(path.as_path()) {
                    let ch = w_chan.clone();
                    scope.execute(move || {
                        // println!("in scope execute for {:?}....", path);
                        // if let Ok(map) = Mmap::open_path(path, Protection::Read) {
                        //     let buf = unsafe { map.as_slice() };
                        //     let res = search::search(rx, &opts, path, buf);
                        //     ch.send(res).unwrap();
                        // }
                        let r = check::check_path(path.as_path(),
                                                clean_f,
                                                !count_f,
                                                if tabs_f { clean::TabStrategy::Tabify } else { clean::TabStrategy::Untabify })
                            .ok()
                            .expect(&format!("check_path for {:?} should work", path));
                        // println!("sending result for {:?}....", path);
                        ch.send(r).unwrap();
                    });
                    // let r = check::check_path(path.as_path(),
                    //                         args.flag_clean,
                    //                         !args.flag_count,
                    //                         if args.flag_tabs { clean::TabStrategy::Tabify } else { clean::TabStrategy::Untabify })
                    //     .ok()
                    //     .expect(&format!("check_path for {:?} should work", path));
                    // if (r & check::HAS_TABS) > 0 { had_tabs += 1 }
                    // if (r & check::TRAILING_SPACES) > 0 { had_trailing_ws += 1 }
                    // if (r & check::HAS_ILLEGAL_CHARACTERS) > 0 { had_illegals += 1 }
                }
            }
        });
    });
    while let Ok(r) = r_chan.recv() {
        if (r & check::HAS_TABS) > 0 { had_tabs += 1 }
        if (r & check::TRAILING_SPACES) > 0 { had_trailing_ws += 1 }
        if (r & check::HAS_ILLEGAL_CHARACTERS) > 0 { had_illegals += 1 }
        checked_files += 1;
    }
    if args.flag_count {
        println!("enforcer-error-count: {}", had_tabs + had_illegals + had_trailing_ws);
    }
    if had_tabs + had_illegals + had_trailing_ws > 0
    {
        println!("checked {} files (enforcer_errors!)", checked_files);
        if had_tabs > 0 {println!("   [with TABS:{}]", had_tabs)}
        if had_illegals > 0 {println!("   [with ILLEGAL CHARS:{}]", had_illegals)}
        if had_trailing_ws > 0 {println!("   [with TRAILING SPACES:{}]", had_trailing_ws)}
        std::process::exit(1);
    }
    else
    {
        println!("checked {} files (enforcer_clean!)", checked_files);
        std::process::exit(0);
    }
}

