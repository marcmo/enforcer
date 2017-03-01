extern crate rustc_serialize;
extern crate docopt;
extern crate toml;
extern crate ansi_term;
extern crate term_painter;
extern crate glob;
extern crate walkdir;
#[macro_use]
extern crate log;

pub mod config;
pub mod search;
pub mod check;
pub mod clean;
