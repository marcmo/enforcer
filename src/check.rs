use std;
use std::io;
use std::path::Path;
use std::fs::File;
use std::fs::metadata;
use std::io::prelude::*;
use rustc_serialize::Decodable;
use glob::Pattern;
use toml;

pub const HAS_TABS: u8               = 1 << 0;
pub const TRAILING_SPACES: u8        = 1 << 1;
pub const HAS_ILLEGAL_CHARACTERS: u8 = 1 << 2;

fn check_content<'a>(input: &'a str, filename: &str) -> io::Result<u8> {
    debug!("check content of {}", filename);
    let mut result = 0;
    let mut i: u32 = 0;
    for line in input.lines_any() {
        i += 1;
        if line.ends_with(' ') {
            result |= TRAILING_SPACES;
        }
        if line.contains("\t") {
            result |= HAS_TABS;
        }
        if line.as_bytes().iter().any(|x| *x > 127) {
            println!("non ASCII line [{}]: {} [{}]", i, line, filename);
            result |= HAS_ILLEGAL_CHARACTERS;
        }
    }
    Ok(result)
}

pub fn clean_string(input: &str) -> String {
    let v: Vec<&str> = input
        .lines_any()
        .map(|line| line.trim_right())
        .collect();

    if input.ends_with("\n") {
        v.join("\n") + "\n"
    }
    else {
        v.join("\n")
    }
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

pub fn check_path(path: &Path, clean: bool) -> io::Result<u8> {
    use std::io::ErrorKind;

    let mut f = try!(File::open(path));
    let mut buffer = String::new();
    let mut check = 0;
    if let Err(e) = f.read_to_string(&mut buffer) {
        match e.kind() {
            ErrorKind::InvalidData => {
                check = check | HAS_ILLEGAL_CHARACTERS;
                let _ = report_offending_line(path);
            },
            _ => return Err(e),
        }
    }
    // only check content if we could read the file
    if check == 0 { check = try!(check_content(&buffer, path.to_str().expect("not available"))); }
    if (check & HAS_TABS) > 0 {
        println!("HAS_TABS:[{}]", path.display());
    }
    if (check & TRAILING_SPACES) > 0 {
        println!("TRAILING_SPACES:[{}]", path.display());
        if clean {
            println!("cleaning trailing whitespaces");
            let res_string = clean_string(&buffer);
            let mut file = try!(File::create(path));
            try!(file.write_all(res_string.as_bytes()));
        }
    }
    Ok(check)
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

pub fn read_config<'a>(input: &'a str) -> io::Result<EnforcerCfg> {
    use std::io::{Error, ErrorKind};
    debug!("read_config");
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
    use super::clean_string;
    use super::read_config;
    use super::is_unwanted;
    use super::s;
    use glob::Pattern;
    use super::EnforcerCfg;
    use std::ffi::OsStr;
    use std::path::Component::Normal;

    #[test]
    fn test_clean_does_not_remove_trailing_newline() {
        let content = "1\n2\n3\n4\n5\n";
        let cleaned = clean_string(content);
        assert!(cleaned.eq(content));
    }
    #[test]
    fn test_clean_trailing_whitespace() {
        let content = "1 \n2";
        let cleaned = clean_string(content);
        println!("{:?}", cleaned);
        assert!(cleaned.eq("1\n2"));
    }
    #[test]
    fn test_load_simple_config() {
        let c = include_str!("../samples/.enforcer");
        let cfg = read_config(c).unwrap();
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
        let cfg = read_config(c).unwrap();
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

