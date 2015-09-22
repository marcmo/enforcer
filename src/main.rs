#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate glob;

use docopt::Docopt;
use std::path::Path;

const USAGE: &'static str = "
enforcer for code rules

Usage:
  enforcer <glob> [-c|--clean]
  enforcer (-h | --help)
  enforcer --version

Options:
  -h --help     Show this screen.
  --version     Show version.
  --count       only count found entries
  -c --clean    clean up trailing whitespaces
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_count: bool,
    flag_clean: bool,
    arg_glob: String,
}
const HAS_TABS: u8 = 1 << 0;
const TRAILING_SPACES: u8 = 1 << 1;

fn check_file(path: &Path) -> std::io::Result<u8> {
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::fs::File;
    let mut result = 0;

    let f = try!(File::open(path));
    let reader = BufReader::new(f);
    for line in reader.lines().filter_map(|result| result.ok()) {
        if line.ends_with(' ') {
            result |= TRAILING_SPACES;
        } else if line.contains("\t") {
            result |= HAS_TABS;
        }
        if result != 0 {
            return Ok(result);
        }
    }
    Ok(result)
}

fn clean_file(path: &Path) -> std::io::Result<()> {
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::fs::File;

    trace!("clean_file {}", path.display());
    let mut cleaned : Vec<String> = Vec::new();
    {
        let f = try!(File::open(path));
        trace!("opened file");
        let reader = BufReader::new(f);
        for line in reader.lines().filter_map(|result| result.ok()) {
            cleaned.push(line.trim_right().to_string());
        }
    }

    trace!("joining lines... ");
    let res_string = cleaned.join("\n");
    let mut file = try!(File::create(path));
    trace!("opened file again");
    try!(file.write_all(res_string.as_bytes()));
    trace!("wrote file");
    Ok(())
}

fn main() {
    use glob::glob;
    env_logger::init().unwrap();

    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let pat = args.arg_glob.to_string();
    for entry in glob(&*pat).unwrap() {
        match entry {
            Ok(path) => {
                let check = check_file(path.as_path()).unwrap();
                if (check & HAS_TABS) > 0 {
                    println!("{} had tabs!!!", path.display());
                }
                if (check & TRAILING_SPACES) > 0 {
                    println!("{} had trailing whitespaces!!!", path.display());
                    if args.flag_clean {
                        println!("cleaning trailing whitespaces");
                        clean_file(path.as_path()).unwrap();
                    }
                }
            },
            Err(e) => println!("{:?}", e),
        }
    }
}
