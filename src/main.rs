extern crate enforcer;
extern crate memmap;
extern crate rustc_serialize;
extern crate docopt;
extern crate scoped_pool;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate pbr;
extern crate ansi_term;

use pbr::ProgressBar;
use std::sync::mpsc::{sync_channel,SyncSender};
use std::cmp::max;
use std::thread;
use memmap::{Mmap, Protection};
use enforcer::config;
use enforcer::search;
use enforcer::check;
use enforcer::clean;
use std::fs;
use std::path;
use std::io::Read;
use std::io::Write;
use std::io::stdout;
use docopt::Docopt;
use ansi_term::Colour;
use ansi_term::Style;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const USAGE: &'static str = "
enforcer for code rules

Usage:
  enforcer [-g ENDINGS...] [-c|--clean] [-q|--quiet] [-t|--tabs] [-l <n>|--length=<n>] [-j <N>|--threads=<N>]
  enforcer (-h | --help)
  enforcer (-v | --version)
  enforcer (-s | --status)

Options:
  -g ENDINGS        use these file endings (e.g. \".h\")
  -h --help         Show this screen.
  -v --version      Show version.
  -s --status       Show configuration status.
  -q --quiet        only count found entries
  -c --clean        clean up trailing whitespaces and convert tabs to spaces
  -t --tabs         leave tabs alone (without that tabs are considered wrong)
  -l --length=<n>   max line length [not checked if empty]
  -j --threads=<N>  number of threads [default: 4]
";
#[derive(Debug, RustcDecodable)]
struct Args {
    flag_clean: bool,
    flag_g: Vec<String>,
    flag_version: bool,
    flag_status: bool,
    flag_quiet: bool,
    flag_tabs: bool,
    flag_length: usize,
    flag_threads: usize,
}

#[allow(dead_code)]
fn main() {
    env_logger::init().unwrap();

    let get_cfg = || -> config::EnforcerCfg {
        fn read_enforcer_config() -> std::io::Result<config::EnforcerCfg> {
            let mut cfg_file = try!(fs::File::open(".enforcer"));
            let mut buffer = String::new();
            try!(cfg_file.read_to_string(&mut buffer));
            config::parse_config(&buffer[..])
        }
        read_enforcer_config()
            .unwrap_or(config::default_cfg())
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
    let cfg_ignores: Vec<String> = enforcer_cfg.ignore;
    let cfg_endings = enforcer_cfg.endings;
    let file_endings = if args.flag_g.len() > 0 {
        args.flag_g
    } else {
        cfg_endings
    };

    let mut checked_files: u32 = 0;
    let mut had_tabs: u32 = 0;
    let mut had_trailing_ws: u32 = 0;
    let mut had_illegals: u32 = 0;
    let mut had_too_long_lines: u32 = 0;
    let clean_f = args.flag_clean;
    let quiet_f = args.flag_quiet;
    let tabs_f = args.flag_tabs;
    let thread_count = max(args.flag_threads, 1);
    let max_line_length = if args.flag_length > 0 { Some(args.flag_length) } else { None };
    if !quiet_f { print!("finding matches...\r") }
    stdout().flush().unwrap();
    let paths = search::find_matches(path::Path::new("."), &cfg_ignores, &file_endings);
    let count: u64 = paths.len() as u64;
    let mut pb = ProgressBar::new(count);

    let (w_chan, r_chan) = sync_channel(thread_count);
    thread::spawn(move || {
        use scoped_pool::Pool;
        let pool = Pool::new(thread_count);

        print!("starting with {} threads....\r", thread_count);
        stdout().flush().unwrap();
        pool.scoped(|scope| {
            for path in paths {
                let ch: SyncSender<u8> = w_chan.clone();
                scope.execute(move || {
                    if !check::is_dir(path.as_path()) {
                        let p = path.clone();
                        match Mmap::open_path(path, Protection::Read) {
                            Ok(map) => {
                                let buf = unsafe { map.as_slice() };
                                let r = check::check_path(p.as_path(),
                                                        buf,
                                                        clean_f,
                                                        !quiet_f,
                                                        max_line_length,
                                                        if tabs_f { clean::TabStrategy::Tabify } else { clean::TabStrategy::Untabify })
                                    .ok()
                                    .expect(&format!("check_path for {:?} should work", p));
                                ch.send(r).unwrap();
                            }
                            Err(e) => {
                                let len = match fs::metadata(p.clone()) {
                                   Ok(metadata)  => { metadata.len() }
                                   Err(_) => {panic!("mmap read error: {}", e)}
                                };
                                if len == 0 {
                                    ch.send(0).unwrap();
                                } else {
                                    panic!("unexpected result for {:?}", p);
                                }
                            }
                        }
                    }
                });
            }
        });
    });
    for _ in 0..count {
        match r_chan.recv() {
            Ok(r) => {
                if (r & check::HAS_TABS) > 0 { had_tabs += 1 }
                if (r & check::TRAILING_SPACES) > 0 { had_trailing_ws += 1 }
                if (r & check::HAS_ILLEGAL_CHARACTERS) > 0 { had_illegals += 1 }
                if (r & check::LINE_TOO_LONG) > 0 { had_too_long_lines += 1 }
            }
            Err(e) => { panic!("error: {}", e); }
        }
        checked_files += 1;
        if quiet_f {pb.inc();}
    }
    if quiet_f {pb.finish();};
    if args.flag_quiet {
        let total_errors = had_tabs + had_illegals + had_trailing_ws + had_too_long_lines;
        println!("{}: {}",
                Style::new().bold().paint("enforcer-error-count"),
                if total_errors > 0 {
                    Colour::Red.bold().paint(format!("{}", total_errors))
                } else {
                    Style::new().bold().paint(format!("{}", total_errors))
                });
    }
    if had_tabs + had_illegals + had_trailing_ws + had_too_long_lines > 0
    {
        println!("checked {} files {}",
                 Style::new().bold().paint(format!("{}", checked_files)),
                 Colour::Red.bold().paint("(enforcer_errors!)"));
        if had_tabs > 0 { println!("   [with TABS:{}]",
                 Colour::Red.bold().paint(format!("{}", had_tabs))) }
        if had_illegals > 0 { println!("   [with ILLEGAL CHARS:{}]",
                 Colour::Red.bold().paint(format!("{}", had_illegals))) }
        if had_trailing_ws > 0 { println!("   [with TRAILING SPACES:{}]",
                 Colour::Red.bold().paint(format!("{}", had_trailing_ws))) }
        if had_too_long_lines > 0 { println!("   [with TOO LONG LINES:{}]",
                 Colour::Red.bold().paint(format!("{}", had_too_long_lines))) }
        std::process::exit(1);
    }
    else
    {
        println!("checked {} files {}", checked_files, Colour::Green.bold().paint("(enforcer_clean!)"));
        std::process::exit(0);
    }
}

