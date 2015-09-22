extern crate rustc_serialize;
extern crate docopt;
extern crate glob;

use docopt::Docopt;
use std::path::Path;

const USAGE: &'static str = "
enforcer for code rules

Usage:
  enforcer <glob>
  enforcer (-h | --help)
  enforcer --version

Options:
  -h --help     Show this screen.
  --version     Show version.
  --count       only count found entries
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_count: bool,
    arg_glob: String,
}

fn check_file(path: &Path) -> std::io::Result<bool> {
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::fs::File;

    let f = try!(File::open(path));
    let reader = BufReader::new(f);
    for line in reader.lines().filter_map(|result| result.ok()) {
        if line.ends_with(' ') {
        	println!("line ends with space");
        	return Ok(false);
        } else if line.contains("\t") {
        	println!("file contains tabs");
        	return Ok(false);
		}
	}
    Ok(false)
}

fn main() {
    use glob::glob;
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let pat = args.arg_glob.to_string();
    for entry in glob(&*pat).unwrap() {
        match entry {
            Ok(path) => {
                println!("{:?}...", path.display());
                let has_tabs = check_file(path.as_path()).unwrap();
                if has_tabs {
                    println!("{} is offending (contains tabs)", path.display());
                }

            },
            Err(e) => println!("{:?}", e),
        }
    }
}
