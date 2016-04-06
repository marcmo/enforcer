extern crate memmap;
use std;
use std::io;
use std::path::Path;
use std::fs::File;
use std::fs::metadata;
use std::io::prelude::*;
use rustc_serialize::Decodable;
use glob::Pattern;
use toml;
use clean;
use self::memmap::{Mmap, Protection};

pub const HAS_TABS: u8               = 1 << 0;
pub const TRAILING_SPACES: u8        = 1 << 1;
pub const HAS_ILLEGAL_CHARACTERS: u8 = 1 << 2;

fn check_content<'a>(input: &'a str, filename: &str, verbose: bool, s: clean::TabStrategy) -> io::Result<u8> {
    debug!("check content of {}", filename);
    let mut result = 0;
    let mut i: u32 = 0;
    for line in input.lines() {
        i += 1;
        if line.ends_with(' ') || line.ends_with('\t') {
            result |= TRAILING_SPACES;
        }
        if s == clean::TabStrategy::Untabify && line.contains("\t") {
            result |= HAS_TABS;
        }
        if line.as_bytes().iter().any(|x| *x > 127) {
            if verbose {println!("non ASCII line [{}]: {} [{}]", i, line, filename)}
            result |= HAS_ILLEGAL_CHARACTERS;
        }
    }
    Ok(result)
}

pub fn is_dir(path: &Path) -> bool {
    if let Ok(result) = metadata(&path) {
        result.is_dir()
    } else {
        false
    }
}

fn report_offending_line(path: &Path) -> std::io::Result<()> {
    use std::io::BufReader;
    let mut i: u32 = 1;
    let f = try!(File::open(path));
    let file = BufReader::new(f);
    for line in file.lines() {
        match line.ok() {
            Some(_) => i = i + 1,
            None => println!("offending line {} in file [{}]", i, path.display()),
        }
    }
    Ok(())
}

pub fn check_path(path: &Path, clean: bool, verbose: bool, s: clean::TabStrategy) -> io::Result<u8> {
    let mut check = 0;
    if let Ok(map) = Mmap::open_path(path, Protection::Read) {
        let buf = unsafe { map.as_slice() };
        match std::str::from_utf8(buf) {
            Err(_) => {
                check = check | HAS_ILLEGAL_CHARACTERS;
                if verbose {let _ = report_offending_line(path);}
                return Ok(check)
            },
            Ok(buffer) => {
                if check == 0 { check = try!(check_content(&buffer, path.to_str().expect("not available"), verbose, s)); }
                if clean {
                    let no_trailing_ws = if (check & TRAILING_SPACES) > 0 {
                        if verbose {println!("TRAILING_SPACES:[{}] -> removing", path.display())}
                        clean::remove_trailing_whitespaces(buffer)
                    } else { buffer.to_string() };
                    let res_string = if (check & HAS_TABS) > 0 {
                        if verbose {println!("HAS_TABS:[{}] -> converting to spaces", path.display())}
                        clean::space_tabs_conversion(no_trailing_ws, clean::TabStrategy::Untabify)
                    } else { no_trailing_ws };
                    let mut file = try!(File::create(path));
                    try!(file.write_all(res_string.as_bytes()));
                }
                else /* report only */ { if verbose {report(check, &path)} }
            },
        };
    };
    // only check content if we could read the file
    Ok(check)
}

fn report(check: u8, path: &Path) -> () {
    if (check & HAS_TABS) > 0 && (check & TRAILING_SPACES) > 0 {
        println!("HAS_TABS && TRAILING_SPACES:[{}]", path.display());
    } else if (check & HAS_TABS) > 0 {
        println!("HAS_TABS:[{}]", path.display());
    } else if (check & TRAILING_SPACES) > 0 {
        println!("TRAILING_SPACES:[{}]", path.display());
    }
}

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct EnforcerCfg {
    pub ignore: Vec<String>,
    pub globs: Vec<String>,
}

pub fn s(x: &str) -> String { x.to_string() }

pub fn default_cfg() -> EnforcerCfg {
    EnforcerCfg {
        ignore: vec![s(".git"), s(".bake"), s(".repo")],
        globs : vec![s("**/*.c"), s("**/*.cpp"), s("**/*.h")],
    }
}

pub fn is_unwanted(comp: std::path::Component, to_ignore: &Vec<String>) -> bool {
    let path_elem = comp.as_os_str()
                        .to_str()
                        .expect(&format!("illegal path component {:?}", comp)[..]);
    to_ignore.iter()
        .any(|x| Pattern::new(x)
            .ok()
            .expect(&format!("{} seems not to be a valid pattern", x)[..])
            .matches(path_elem))
}

pub fn parse_config<'a>(input: &'a str) -> io::Result<EnforcerCfg> {
    use std::io::{Error, ErrorKind};
    debug!("parse_config");
    let mut parser = toml::Parser::new(input);
    fn default_err() -> Error {
        Error::new(ErrorKind::InvalidData, "could not parse the config")
    }

    parser.parse().map_or(Err(default_err()), |toml| {
        let mut decoder = toml::Decoder::new(toml::Value::Table(toml));
        EnforcerCfg::decode(&mut decoder)
            .ok()
            .map_or(Err(default_err()), |config|
                Ok(config))
    })
}

#[cfg(test)]
mod tests {
    use super::check_content;
    use super::TRAILING_SPACES;
    use super::HAS_TABS;
    use super::HAS_ILLEGAL_CHARACTERS;
    use super::parse_config;
    use super::is_unwanted;
    use super::s;
    use glob::Pattern;
    use super::EnforcerCfg;
    use clean::TabStrategy::Untabify;
    use clean::TabStrategy::Tabify;
    use std::ffi::OsStr;
    use std::path::Component::Normal;
    #[test]
    fn test_check_good_content() {
        let content = " 1\n";
        let res = check_content(content, "foo.h", false, Untabify);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_good_content_with_tabs() {
        let content = "\t1\n";
        let res = check_content(content, "foo.h", false, Tabify);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_bad_content_with_tabs() {
        let content = "\t1\n";
        let res = check_content(content, "foo.h", false, Untabify);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 1);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_content_trailing_ws() {
        let content = "1 \n";
        let res = check_content(content, "foo.h", false, Untabify);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) > 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_content_trailing_tabs() {
        let content = "1\t\n";
        let res = check_content(content, "foo.h", false, Untabify);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) > 0);
        assert!((check & HAS_TABS) > 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_load_simple_config() {
        let c = include_str!("../samples/.enforcer");
        let cfg = parse_config(c).unwrap();
        assert_eq!(cfg.ignore.len(), 2);
        let expected = EnforcerCfg {
            ignore: vec![s(".git"), s(".repo")],
            globs : vec![s("**/*.c"), s("**/*.cpp"), s("**/*.h")],
        };
        assert_eq!(expected.ignore, cfg.ignore);
        assert_eq!(expected, cfg);
    }
    #[test]
    fn test_load_broken_config() {
        let c = include_str!("../samples/.enforcer_broken");
        let cfg = parse_config(c).unwrap();
        let expected = EnforcerCfg {
            ignore: vec![s(".git"), s(".repo")],
            globs : vec![s("**/*.c"), s("**/*.cpp"), s("**/*.h")],
        };
        assert!(expected.ignore != cfg.ignore);
    }
    #[test]
    fn test_glob() {
        assert!(Pattern::new("build_*").unwrap().matches("build_Debug"));
    }
    #[test]
    fn test_is_unwanted() {
        let cfg = EnforcerCfg { ignore: vec![s("build_*"), s(".git")], globs: vec![]};
        assert!(is_unwanted(Normal(OsStr::new("build_")), &cfg.ignore));
        assert!(is_unwanted(Normal(OsStr::new("build_Debug")), &cfg.ignore));
        assert!(is_unwanted(Normal(OsStr::new(".git")), &cfg.ignore));
        assert!(!is_unwanted(Normal(OsStr::new("bla")), &cfg.ignore));
    }
}

