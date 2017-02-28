extern crate enforcer;
extern crate memmap;
extern crate rustc_serialize;
extern crate docopt;
extern crate scoped_pool;
extern crate log;
extern crate env_logger;
extern crate pbr;
extern crate ansi_term;

use pbr::ProgressBar;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::cmp::max;
use std::thread;
use memmap::{Mmap, Protection};
use enforcer::config;
use enforcer::search;
use enforcer::check;
use enforcer::clean;
use std::fs;
use std::path;
use docopt::Docopt;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const USAGE: &'static str =
"
enforcer for code rules

Usage:
  enforcer [-g ENDINGS...] [-c|--clean] [-q|--quiet] \
      [-t|--tabs] [-l <n>|--length=<n>] [-j <N>|--threads=<N>] \
    [-a|--color]
  enforcer (-h | --help)
  enforcer (-v | --version)
  enforcer (-s | --status)

Options:
  -g ENDINGS        use these file endings (e.g. \".h\").
  -h --help         show this screen.
  -v --version      show version.
  -s --status       show configuration status.
  -q --quiet        only count found entries.
  -c --clean        clean up trailing whitespaces and convert tabs to spaces.
  -t --tabs         leave tabs alone (without that tabs are considered wrong).
  -l --length=<n>   max line length [not checked if empty].
  -j --threads=<N>  number of threads [default: 4].
  -a --color        use ANSI colored output
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
    flag_color: bool,
}

#[allow(dead_code)]
fn main() {
    env_logger::init().unwrap();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    if args.flag_version {
        println!("  Version: {}", VERSION);
        std::process::exit(0);
    }
    let enforcer_cfg = config::get_cfg();
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
    let color_f = args.flag_color;
    let max_line_length = if args.flag_length > 0 {
        Some(args.flag_length)
    } else {
        None
    };
    let paths = search::find_matches(path::Path::new("."), &cfg_ignores, &file_endings);
    let count: u64 = paths.len() as u64;
    let mut pb = ProgressBar::new(count);
    // logger thread
    let (logging_tx, logging_rx) = sync_channel::<Option<String>>(0);
    let stop_logging_tx = logging_tx.clone();
    thread::spawn(move || {
        let mut done = false;
        while !done {
            done = logging_rx.recv()
                //convert to option
                .ok()
                // not done when we got a receive error (sender end of connection closed)
                .map_or(false, |maybe_print|
                        maybe_print
                        // a None indicates that logging is done
                        .map_or(true, |p|
                                // just print the string we received
                                {print!("{}", p); false}));
        }
    });

    let (w_chan, r_chan) = sync_channel(thread_count);
    thread::spawn(move || {
        use scoped_pool::Pool;
        let pool = Pool::new(thread_count);

        pool.scoped(|scope| {
            for path in paths {
                let ch: SyncSender<Result<u8, std::io::Error>> = w_chan.clone();
                let l_ch: SyncSender<Option<String>> = logging_tx.clone();
                scope.execute(move || {
                    if !check::is_dir(path.as_path()) {
                        let p = path.clone();
                        match Mmap::open_path(path, Protection::Read) {
                            Ok(map) => {
                                let buf = unsafe { map.as_slice() };
                                let r = check::check_path(
                                    p.as_path(),
                                    buf,
                                    clean_f,
                                    !quiet_f,
                                    max_line_length,
                                    if tabs_f {
                                        clean::TabStrategy::Tabify
                                    } else {
                                        clean::TabStrategy::Untabify
                                    },
                                    l_ch);
                                ch.send(r).unwrap();
                            }
                            Err(e) => {
                                let len = match fs::metadata(p.clone()) {
                                    Ok(metadata) => metadata.len(),
                                    Err(_) => panic!("mmap read error: {}", e),
                                };
                                if len == 0 {
                                    ch.send(Ok(0)).unwrap();
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
            Ok(res) => {
                match res {
                    Ok(r)  => {
                        if (r & check::HAS_TABS) > 0 {
                            had_tabs += 1
                        }
                        if (r & check::TRAILING_SPACES) > 0 {
                            had_trailing_ws += 1
                        }
                        if (r & check::HAS_ILLEGAL_CHARACTERS) > 0 {
                            had_illegals += 1
                        }
                        if (r & check::LINE_TOO_LONG) > 0 {
                            had_too_long_lines += 1
                        }
                    }
                    Err(e) => {
                        println!("error occured here: {}", e);
                    }
                }
            }
            Err(e) => {
                panic!("error in channel: {}", e);
            }
        }
        checked_files += 1;
        if quiet_f {
            pb.inc();
        }
    }
    if quiet_f {
        pb.finish();
    };
    let _ = stop_logging_tx.send(None);
    if args.flag_quiet {
        let total_errors = had_tabs + had_illegals + had_trailing_ws + had_too_long_lines;
        if color_f{
            println!("{}: {}",
                     check::bold("enforcer-error-count"),
                     total_errors);
        } else {
            println!("enforcer-error-count: {}", total_errors);
        }
    }
    if had_tabs + had_illegals + had_trailing_ws + had_too_long_lines > 0 {
        if color_f{
            println!("checked {} files {}",
                     checked_files,
                     check::bold("(enforcer_errors!)"));
        } else {
            println!("checked {} files (enforcer_errors!)", checked_files);
        }
        if had_tabs > 0 {
            println!("   [with TABS:{}]", had_tabs)
        }
        if had_illegals > 0 {
            println!("   [with ILLEGAL CHARS:{}]", had_illegals)
        }
        if had_trailing_ws > 0 {
            println!("   [with TRAILING SPACES:{}]", had_trailing_ws)
        }
        if had_too_long_lines > 0 {
            println!("   [with TOO LONG LINES:{}]", had_too_long_lines)
        }
        std::process::exit(1);
    } else {
        if color_f{
            println!("checked {} files {}",
                     checked_files,
                     check::green("(enforcer_clean!)"));
        } else {
            println!("checked {} files (enforcer_clean!)", checked_files);
        }


        std::process::exit(0);
    }
}
