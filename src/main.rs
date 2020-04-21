extern crate scoped_pool;
#[macro_use]
extern crate log;
extern crate ansi_term;
extern crate env_logger;
extern crate pbr;
extern crate serde_derive;
extern crate term_painter;
#[macro_use]
extern crate clap;
extern crate anyhow;
extern crate glob;
extern crate num_cpus;
extern crate regex;
extern crate toml;
extern crate unic_char_range;
extern crate walkdir;

use args::Args;

mod app;
mod args;
mod check;
mod clean;
mod config;
mod search;

use pbr::ProgressBar;
use std::{
    fs::File,
    io::prelude::*,
    num, process,
    sync::{
        mpsc::{sync_channel, SyncSender},
        Arc,
    },
    thread,
};

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
    let file_endings = if !args.endings().is_empty() {
        args.endings()
    } else {
        &cfg_endings
    };

    let mut checked_files: u32 = 0;
    let mut had_tabs: u32 = 0;
    let mut had_trailing_ws: u32 = 0;
    let mut had_illegals: u32 = 0;
    let mut had_too_long_lines: u32 = 0;
    let mut had_win_line_endings: u32 = 0;
    let clean_f = args.clean();
    let tabs_f = args.tabs();
    let use_crlf = args.use_crlf();
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
            done = logging_rx
                .recv()
                .ok()
                // not done when we got a receive error (sender end of connection closed)
                .map_or(false, |maybe_print| {
                    maybe_print
                        // a None indicates that logging is done
                        .map_or(true, |p|
                                // just print the string we received
                                {print!("{}", p); false})
                });
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
                        let mut f = File::open(path)
                            .unwrap_or_else(|_| panic!("error reading file {:?}", p));
                        let mut buffer = Vec::new();
                        f.read_to_end(&mut buffer)
                            .unwrap_or_else(|_| panic!("error reading file {:?}", p));

                        let r = check::check_path(
                            p.as_path(),
                            &buffer,
                            clean_f,
                            info_level,
                            max_line_length,
                            if tabs_f {
                                clean::TabStrategy::Tabify
                            } else {
                                clean::TabStrategy::Untabify
                            },
                            if use_crlf {
                                clean::LineEnding::CRLF
                            } else {
                                clean::LineEnding::LF
                            },
                            l_ch,
                        );
                        ch.send(r).expect("send result with SyncSender");
                    }
                });
            }
        });
    });
    for _ in 0..count {
        match r_chan.recv() {
            Ok(res) => match res {
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
                    if (r & check::HAS_WINDOWS_LINE_ENDINGS) > 0 {
                        had_win_line_endings += 1
                    }
                }
                Err(e) => {
                    error!("error occured here: {}", e);
                }
            },
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
    let findings = Findings {
        had_tabs,
        had_trailing_ws,
        had_illegals,
        had_too_long_lines,
        had_win_line_endings,
        checked_files,
    };
    report_findings(info_level == check::InfoLevel::Quiet, findings, color_f)
}

#[derive(Debug)]
struct Findings {
    had_tabs: u32,
    had_trailing_ws: u32,
    had_illegals: u32,
    had_too_long_lines: u32,
    had_win_line_endings: u32,
    checked_files: u32,
}

fn report_findings(
    quiet: bool,
    findings: Findings,
    colored: bool,
) -> Result<u64, num::ParseIntError> {
    let total_errors = findings.had_tabs
        + findings.had_illegals
        + findings.had_trailing_ws
        + findings.had_too_long_lines
        + findings.had_win_line_endings;
    if quiet {
        if colored {
            println!("{}: {}", check::bold("enforcer-error-count"), total_errors);
        } else {
            println!("enforcer-error-count: {}", total_errors);
        }
    }
    if total_errors > 0 {
        if colored {
            println!(
                "checked {} files {}",
                findings.checked_files,
                check::bold("(enforcer_errors!)")
            );
        } else {
            println!(
                "checked {} files (enforcer_errors!)",
                findings.checked_files
            );
        }
        if findings.had_tabs > 0 {
            println!("   [with TABS:{}]", findings.had_tabs)
        }
        if findings.had_illegals > 0 {
            println!("   [with ILLEGAL CHARS:{}]", findings.had_illegals)
        }
        if findings.had_trailing_ws > 0 {
            println!("   [with TRAILING SPACES:{}]", findings.had_trailing_ws)
        }
        if findings.had_too_long_lines > 0 {
            println!("   [with TOO LONG LINES:{}]", findings.had_too_long_lines)
        }
        if findings.had_win_line_endings > 0 {
            println!(
                "   [with WINDOWS LINE ENDINGS:{}]",
                findings.had_win_line_endings
            )
        }
        Ok(1)
    } else {
        if colored {
            println!(
                "checked {} files {}",
                findings.checked_files,
                check::green("(enforcer_clean!)")
            );
        } else {
            println!("checked {} files (enforcer_clean!)", findings.checked_files);
        }
        Ok(0)
    }
}
