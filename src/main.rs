extern crate enforcer;
extern crate rustc_serialize;
extern crate docopt;
extern crate glob;
#[macro_use] extern crate log;
extern crate env_logger;

extern crate time;
extern crate threadpool;
use time::PreciseTime;
use enforcer::check;
use enforcer::clean;
use std::fs::File;
use std::io::Read;
use docopt::Docopt;
use threadpool::ThreadPool;
use std::sync::mpsc::channel;
use std::thread;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const USAGE: &'static str = "
enforcer for code rules

Usage:
  enforcer [-g GLOB...] [-c|--clean] [-n|--count] [-t|--tabs]
  enforcer (-h | --help)
  enforcer (-v | --version)
  enforcer (-s | --status)

Options:
  -g GLOB       use these glob patterns (e.g. \"**/*.h\")
  -h --help     Show this screen.
  -v --version  Show version.
  -s --status   Show configuration status.
  -n --count    only count found entries
  -c --clean    clean up trailing whitespaces
  -t --tabs     leave tabs alone (without that tabs are considered wrong)
";
#[derive(Debug, RustcDecodable)]
struct Args {
    flag_clean: bool,
    flag_g: Vec<String>,
    flag_version: bool,
    flag_status: bool,
    flag_count: bool,
    flag_tabs: bool,
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
    #[derive(Debug)]
    struct FileResult {
        had_tabs: u32,
        had_trailing_ws: u32,
        had_illegals: u32,
    }

    let start = PreciseTime::now();
    // whatever you want to do
    println!("before find_matches");
    let paths = find_matches();
    let end = PreciseTime::now();
    println!("{} after find_matches", start.to(end));

    println!("before check_path ({} files)", paths.len());
    let start2 = PreciseTime::now();

    let cl = args.flag_clean;
    let cnt = !args.flag_count;
    let tabs = args.flag_tabs;
    let pool = ThreadPool::new(10);
    let (tx, rx) = channel();
    for path in paths {
        if !check::is_dir(path.as_path()) {
            checked_files += 1;
            let tx = tx.clone();
            pool.execute(move|| {
                println!("thread checking {:?}", path.as_path());
                let r = check::check_path(path.as_path(), cl, cnt,
                                        if tabs { clean::TabStrategy::Tabify } else { clean::TabStrategy::Untabify })
                    .ok()
                    .expect(&format!("check_path for {:?} should work", path));
                let mut res = FileResult{had_tabs: 0, had_illegals: 0, had_trailing_ws: 0};
                if (r & check::HAS_TABS) > 0 { res.had_tabs += 1 }
                if (r & check::TRAILING_SPACES) > 0 { res.had_trailing_ws += 1 }
                if (r & check::HAS_ILLEGAL_CHARACTERS) > 0 { res.had_illegals += 1 }
                println!("sending back: {:?}", res);
                tx.send(res).unwrap();
            });
            println!("currently active threads: {}", pool.active_count());
        }
    }
    let collector = thread::spawn(move || {
        let (a,b,c) = rx.iter().take(10).fold((0,0,0), |(x,y,z), res| (x+res.had_tabs, y+res.had_illegals, z+res.had_trailing_ws));
    });
    // let thread_res = rx.recv().unwrap();
    let end2 = PreciseTime::now();
    println!("{} after check_path (res was: {:?})", start2.to(end2), (a,b,c));
    if args.flag_count {
        println!("enforcer-error-count: {}", a + b + c);
    }
    if a + b + c > 0
    {
        println!("checked {} files (enforcer_errors!)", checked_files);
        if a > 0 {println!("   [with TABS:{}]", a)}
        if b > 0 {println!("   [with ILLEGAL CHARS:{}]", b)}
        if c > 0 {println!("   [with TRAILING SPACES:{}]", c)}
        std::process::exit(1);
    }
    else
    {
        println!("checked {} files (enforcer_clean!)", checked_files);
        std::process::exit(0);
    }
}

