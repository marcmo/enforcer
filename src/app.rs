use clap::{App, Arg};

const ABOUT: &'static str = "
enforcer is a utility to help you keep your source code in a more consistent state.
It will check all matching files in a specified directory and report on any
inconsistencies. Some of those can also be fixed automatically (like conversion
from tabs to spaces), others need manual attention (like handling too long lines).

Project home page: https://github.com/marcmo/enforcer

Use -h for short descriptions.";

const USAGE: &'static str = "
    enforcer [OPTIONS] [-g ENDINGS...] <path>
    enforcer [-g ENDINGS...] [-q | --quiet] [-j <NUM> | --threads=<NUM>] [-a | --color] <path>
    enforcer [-c | --clean] <path>
    enforcer [-l <MAX> | --length=<MAX>] <path>";

const TEMPLATE: &'static str = "\
{bin} {version}
{author}
{about}

USAGE:{usage}

ARGS:
{positionals}

OPTIONS:
{unified}";

pub fn app() -> App<'static, 'static> {
    App::new("enforcer")
        .author(crate_authors!())
        .version(crate_version!())
        .about(ABOUT)
        .usage(USAGE)
        .template(TEMPLATE)
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("verbosity level"))
        .arg(Arg::with_name("debug")
            .long("debug")
            .help("only use to show debug output"))
        .arg(Arg::with_name("path").multiple(true))
        .arg(Arg::with_name("endings")
            .short("g")
            .value_name("ENDINGS")
            .help("use these file endings (e.g. \".cpp\",\".h\")")
            .takes_value(true))
        .arg(Arg::with_name("clean")
            .short("c")
            .long("clean")
            .help("clean up trailing whitespaces and convert tabs to spaces")
            .takes_value(false))
        .arg(Arg::with_name("config-path")
            .short("f")
            .long("config-file")
            .value_name("CONFIG")
            .help("path to configuration file")
            .takes_value(true))
        .arg(Arg::with_name("status")
            .short("s")
            .long("config-status")
            .value_name("FILE")
            .help("check the configuration that is used")
            .takes_value(false))
        .arg(Arg::with_name("quiet")
            .short("q")
            .long("quiet")
            .help("only count found entries")
            .takes_value(false))
        .arg(Arg::with_name("color")
            .short("a")
            .long("color")
            .help("use ANSI colored output")
            .takes_value(false))
        .arg(Arg::with_name("tabs")
            .short("t")
            .long("tabs")
            .help("leave tabs alone (without that tabs are considered wrong)")
            .takes_value(false))
        .arg(Arg::with_name("L")
            .value_name("MAX").takes_value(true)
            .short("l")
            .long("length")
            .help("max line length [not checked if empty]")
            .validator(validate_number))
        .arg(Arg::with_name("N")
            .value_name("NUM").takes_value(true)
            .short("j")
            .long("threads")
            .default_value("4")
            .help("number of threads")
            .validator(validate_number))
}

fn validate_number(s: String) -> Result<(), String> {
    s.parse::<usize>().map(|_|()).map_err(|err| err.to_string())
}

