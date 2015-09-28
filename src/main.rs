#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate glob;

use docopt::Docopt;
use std::path::Path;
use std::fs::File;
use std::fs::metadata;
use std::io::prelude::*;

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
const HAS_TABS: u8               = 1 << 0;
const TRAILING_SPACES: u8        = 1 << 1;
const HAS_ILLEGAL_CHARACTERS: u8 = 1 << 2;


fn check_content<'a>(input: &'a str) -> std::io::Result<u8> {
    let mut result = 0;
    for line in input.lines_any() {
        if line.ends_with(' ') {
            result |= TRAILING_SPACES;
        }
        if line.contains("\t") {
            result |= HAS_TABS;
        }
        if line.as_bytes().iter().any(|x| *x > 127) {
            result |= HAS_ILLEGAL_CHARACTERS;
        }
    }
    Ok(result)
}

// fn remove_spaces<'a>(input: &'a str) -> Cow<'a, str> {
//     if input.contains(' ') {
//         input
//         .chars()
//         .filter(|&x| x != ' ')
//         .collect::<std::string::String>()
//         .into()
//     } else {
//         input.into()
//     }
// }

fn clean_string(input: &str) -> std::io::Result<String> {
    let v: Vec<&str> = input
        .lines_any()
        .map(|line| line.trim_right())
        .collect();

    Ok(if input.ends_with("\n") {
            v.join("\n") + "\n"
        } else {
            v.join("\n")
        })
}

fn is_dir(path: &Path) -> bool {
    if let Ok(result) = metadata(&path) {
        result.is_dir()
    } else {
        false
    }
}

fn check_path(path: &Path, clean: bool) -> std::io::Result<()> {
    use std::io::ErrorKind;

    let mut f = try!(File::open(path));
    let mut buffer = String::new();
    let mut check = 0;
    if let Err(e) = f.read_to_string(&mut buffer) {
        match e.kind() {
            ErrorKind::InvalidData => check = check | HAS_ILLEGAL_CHARACTERS,
            _ => return Err(e),
        }
    }
    // only check content if we could read the file
    if check == 0 { check = try!(check_content(&buffer)); }
    if (check & HAS_ILLEGAL_CHARACTERS) > 0 {
        println!("HAS_ILLEGAL_CHARACTERS:[{}]", path.display());
    }
    if (check & HAS_TABS) > 0 {
        println!("HAS_TABS:[{}]", path.display());
    }
    if (check & TRAILING_SPACES) > 0 {
        println!("TRAILING_SPACES:[{}]", path.display());
        if clean {
            println!("cleaning trailing whitespaces");
            let res_string = try!(clean_string(&buffer));
            let mut file = try!(File::create(path));
            try!(file.write_all(res_string.as_bytes()));
        }
    }
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

    // for line in reader.lines().filter_map(|result| result.ok()) {
    for entry in glob(&*pat).unwrap() {
        match entry {
            Ok(path) => if !is_dir(path.as_path()) {
                            check_path(path.as_path(), args.flag_clean)
                                .ok()
                                .expect(&format!("check_path for {:?} should work", path));
                        },
            Err(e) => println!("{:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::clean_string;

    #[test]
    fn test_clean_does_not_remove_trailing_newline() {
        let content = "1\n2\n3\n4\n5\n";
        let cleaned = clean_string(content).unwrap();
        assert!(cleaned.eq(content));
    }
    #[test]
    fn test_clean_trailing_whitespace() {
        let content = "1 \n2";
        let cleaned = clean_string(content).unwrap();
        println!("{:?}", cleaned);
        assert!(cleaned.eq("1\n2"));
    }
}

