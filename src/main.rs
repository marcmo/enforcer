extern crate rustc_serialize;
extern crate scoped_pool;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate pbr;
extern crate term_painter;
extern crate ansi_term;
#[macro_use]
extern crate clap;
extern crate num_cpus;
extern crate toml;
extern crate glob;
extern crate walkdir;
extern crate regex;
extern crate unic_char_range;

use args::Args;

mod app;
mod args;
mod check;
mod clean;
mod config;
mod search;

use std::num;
use std::process;
use std::sync::Arc;
use pbr::ProgressBar;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;
use std::fs::File;
use std::io::prelude::*;

macro_rules! eprintln {
    ($($tt:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(&mut ::std::io::stderr(), $($tt)*);
    }}
}

#[allow(dead_code)]
fn main() {

    match Args::parse().map(Arc::new).and_then(run) {
        Ok(0) => process::exit(0),
        Ok(_) => process::exit(1),
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}

fn run(args: Arc<Args>) -> Result<u64, num::ParseIntError> {
    let enforcer_cfg = config::get_cfg(args.config_file());
    if args.status() {
        println!("  using this config: {:?}", enforcer_cfg);
        std::process::exit(0);
    }
    let cfg_ignores: &Vec<String> = &enforcer_cfg.ignore;
    let cfg_endings = enforcer_cfg.endings;
    let file_endings = if args.endings().len() > 0 {
        args.endings()
    } else {
        &cfg_endings
    };

    let mut checked_files: u32 = 0;
    let mut had_tabs: u32 = 0;
    let mut had_trailing_ws: u32 = 0;
    let mut had_illegals: u32 = 0;
    let mut had_too_long_lines: u32 = 0;
    let clean_f = args.clean();
    let tabs_f = args.tabs();
    let thread_count = args.threads();
    let color_f = args.color();
    let max_line_length = args.line_length();
    let start_dir = args.path();
    debug!("args:{:?}", args);
    if args.quiet() {
        println!("quiet flag was used but is deprecated...use verbosity instead");
    }
    let info_level: check::InfoLevel = args.info_level();
    let paths = search::find_matches(start_dir.as_path(), cfg_ignores, file_endings);
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

        pool.scoped(|scope| for path in paths {
            let ch: SyncSender<Result<u8, std::io::Error>> = w_chan.clone();
            let l_ch: SyncSender<Option<String>> = logging_tx.clone();
            scope.execute(move || if !check::is_dir(path.as_path()) {
                let p = path.clone();
                let mut f = File::open(path).expect(format!("error reading file {:?}", p).as_str());
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer).expect(format!("error reading file {:?}", p).as_str());

                let r = check::check_path(p.as_path(),
                &buffer,
                clean_f,
                info_level,
                max_line_length,
                if tabs_f {
                    clean::TabStrategy::Tabify
                } else {
                    clean::TabStrategy::Untabify
                },
                l_ch);
                ch.send(r).expect("send result with SyncSender");
            });
        });
    });
    for _ in 0..count {
        match r_chan.recv() {
            Ok(res) => {
                match res {
                    Ok(r) => {
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
                        error!("error occured here: {}", e);
                    }
                }
            }
            Err(e) => {
                panic!("error in channel: {}", e);
            }
        }
        checked_files += 1;
        if info_level == check::InfoLevel::Quiet {
            pb.inc();
        }
    }
    if info_level == check::InfoLevel::Quiet {
        pb.finish();
    };
    let _ = stop_logging_tx.send(None);
    if info_level == check::InfoLevel::Quiet {
        let total_errors = had_tabs + had_illegals + had_trailing_ws + had_too_long_lines;
        if color_f {
            println!("{}: {}", check::bold("enforcer-error-count"), total_errors);
        } else {
            println!("enforcer-error-count: {}", total_errors);
        }
    }
    if had_tabs + had_illegals + had_trailing_ws + had_too_long_lines > 0 {
        if color_f {
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
        Ok(1)
    } else {
        if color_f {
            println!("checked {} files {}",
                     checked_files,
                     check::green("(enforcer_clean!)"));
        } else {
            println!("checked {} files (enforcer_clean!)", checked_files);
        }
        Ok(0)
    }
}
