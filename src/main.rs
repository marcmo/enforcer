#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate glob;

use docopt::Docopt;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;

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
    use std::fs::File;
    let mut result = 0;

    let f = try!(File::open(path));
    let reader = BufReader::new(f);
    for line in reader.lines().filter_map(|result| result.ok()) {
        if line.ends_with(' ') {
            result |= TRAILING_SPACES;
        } else if line.contains("\t") {
            result |= HAS_TABS;
        } else if line.as_bytes().iter().any(|x| *x > 127) {
            println!("file {} line \"{}\" contained illegal characters", path.display(), line);
        }
        if result != 0 {
            return Ok(result);
        }
    }
    Ok(result)
}

fn clean_buf<T: std::io::BufRead + Sized>(reader: T) -> std::io::Result<String> {
    let mut cleaned : Vec<String> = Vec::new();
    for line in reader.lines().filter_map(|result| result.ok()) {
        cleaned.push(line.trim_right().to_string());
    }
    Ok(cleaned.join("\n") + "\n")
}

fn clean_file(path: &Path) -> std::io::Result<()> {
    use std::io::prelude::*;

    let f = try!(File::open(path));
    let reader = BufReader::new(f);
    let res_string = try!(clean_buf(reader));
    let mut file = try!(File::create(path));
    try!(file.write_all(res_string.as_bytes()));
    Ok(())
}

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::clean_buf;

    #[test]
    fn test_clean_does_not_remove_trailing_newline() {
        let content = "1\n2\n3\n4\n5\n";
        let cleaned = clean_buf(content.as_bytes()).unwrap();
        assert!(cleaned.eq(content));
    }
    #[test]
    fn test_clean_trailing_whitespace() {
        let content = "1 \n2";
        let cleaned = clean_buf(content.as_bytes()).unwrap();
        println!("{:?}", cleaned);
        assert!(cleaned.eq("1\n2"));
    }
}

