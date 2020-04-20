use std::cmp;
use std::num;
use std::ops;
use std::process;

use crate::app;
use clap;
use env_logger;
use num_cpus;

use super::check::InfoLevel;
use std::path::{Path, PathBuf};
use std::result::Result;

/// `Args` are transformed/normalized from `ArgMatches`.
#[derive(Debug)]
pub struct Args {
    path: PathBuf,
    endings: Vec<String>,
    clean: bool,
    config_file: Option<PathBuf>,
    line_length: Option<usize>,
    color: bool,
    threads: usize,
    quiet: bool,
    status: bool,
    tabs: bool,
    info_level: InfoLevel,
}

fn init_logging(is_debug: bool) {
    use chrono::Local;
    use std::io::Write;
    if std::env::var("RUST_LOG").is_err() {
        if is_debug {
            std::env::set_var("RUST_LOG", "warn,enforcer=debug");
        } else {
            std::env::set_var("RUST_LOG", "warn,enforcer=info");
        }
    }
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.level(),
                record.args()
            )
        })
        .init();
}

impl Args {
    /// Parse the command line arguments for this process.
    ///
    /// If a CLI usage error occurred, then exit the process and print a usage
    /// or error message. Similarly, if the user requested the version of
    /// enforcer, then print the version and exit.
    ///
    /// Also, initialize a global logger.
    pub fn parse() -> Result<Args, num::ParseIntError> {
        let matches = app::app().get_matches();
        if matches.is_present("help") {
            let _ = app::app().print_help();
            println!();
            process::exit(0);
        }
        if matches.is_present("version") {
            println!("enforcer {}", crate_version!());
            process::exit(0);
        }
        init_logging(matches.is_present("debug"));
        ArgMatches(matches).to_args()
    }

    /// Whether enforcer should be quiet or not.
    pub fn quiet(&self) -> bool {
        self.quiet
    }
    /// Return the path that should be searched.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
    pub fn clean(&self) -> bool {
        self.clean
    }
    pub fn status(&self) -> bool {
        self.status
    }
    pub fn tabs(&self) -> bool {
        self.tabs
    }
    pub fn config_file(&self) -> &Option<PathBuf> {
        &self.config_file
    }

    /// Returns true if and only if enforcer should color its output.
    pub fn color(&self) -> bool {
        self.color
    }

    /// Return the endings that should be searched.
    pub fn endings(&self) -> &[String] {
        &self.endings
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    pub fn line_length(&self) -> Option<usize> {
        self.line_length
    }

    pub fn info_level(&self) -> InfoLevel {
        self.info_level
    }
}

/// `ArgMatches` wraps `clap::ArgMatches` and provides semantic meaning to
/// several options/flags.
struct ArgMatches<'a>(clap::ArgMatches<'a>);

impl<'a> ops::Deref for ArgMatches<'a> {
    type Target = clap::ArgMatches<'a>;
    fn deref(&self) -> &clap::ArgMatches<'a> {
        &self.0
    }
}

impl<'a> ArgMatches<'a> {
    /// Convert the result of parsing CLI arguments into enforcer's
    /// configuration.
    fn to_args(&self) -> Result<Args, num::ParseIntError> {
        let path = self.path();
        let endings = self.endings();
        let config = self.config_path();
        let quiet = self.is_present("quiet");
        let args = Args {
            path,
            endings,
            clean: self.is_present("clean"),
            config_file: config,
            line_length: self.usize_of("L")?,
            color: self.is_present("color"),
            quiet,
            threads: self.threads()?,
            status: self.is_present("status"),
            tabs: self.is_present("tabs"),
            info_level: self.info_level(),
        };
        Ok(args)
    }

    /// Return all file endings that enforcer should search.
    fn endings(&self) -> Vec<String> {
        let endings: Vec<String> = match self.values_of_lossy("endings") {
            None => vec![],
            Some(vals) => vals,
        };
        endings
    }

    /// Return file path that enforcer should search.
    fn path(&self) -> PathBuf {
        match self.value_of_os("path") {
            None => self.default_path(),
            Some(val) => Path::new(val).to_path_buf(),
        }
    }

    /// Return path to config file.
    fn info_level(&self) -> InfoLevel {
        match self.occurrences_of("verbose") {
            0 => InfoLevel::Quiet,
            1 => InfoLevel::Normal,
            _ => InfoLevel::Verbose,
        }
    }

    /// Return path to config file.
    fn config_path(&self) -> Option<PathBuf> {
        match self.value_of_os("config-path") {
            None => None,
            Some(val) => Some(Path::new(val).to_path_buf()),
        }
    }

    /// Return the default path that enforcer should search.
    fn default_path(&self) -> PathBuf {
        Path::new("./").to_path_buf()
    }

    /// Returns the approximate number of threads that enforcer should use.
    fn threads(&self) -> Result<usize, num::ParseIntError> {
        let threads = self.usize_of("N")?.unwrap_or(0);
        Ok(if threads == 0 {
            cmp::min(12, num_cpus::get())
        } else {
            threads
        })
    }

    /// Safely reads an arg value with the given name, and if it's present,
    /// tries to parse it as a usize value.
    fn usize_of(&self, name: &str) -> Result<Option<usize>, num::ParseIntError> {
        match self.0.value_of_lossy(name) {
            None => Ok(None),
            Some(v) => v.parse().map(Some).map_err(From::from),
        }
    }
}
