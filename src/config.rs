use std;
use std::io;
use toml;
use rustc_serialize::Decodable;
use glob::Pattern;

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct EnforcerCfg {
    pub ignore: Vec<String>,
    pub endings: Vec<String>,
}

pub fn s(x: &str) -> String { x.to_string() }

pub fn default_cfg() -> EnforcerCfg {
    EnforcerCfg {
        ignore: vec![s(".git"), s(".bake"), s(".repo")],
        endings : vec![s(".c"), s(".cpp"), s(".h")],
    }
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


#[cfg(test)]
mod tests {
    use super::s;
    use glob::Pattern;
    use super::EnforcerCfg;
    use super::parse_config;
    use super::is_unwanted;
    use std::ffi::OsStr;
    use std::path::Component::Normal;

    #[test]
    fn test_load_simple_config() {
        let c = include_str!("../samples/.enforcer");
        let cfg = parse_config(c).unwrap();
        assert_eq!(cfg.ignore.len(), 2);
        let expected = EnforcerCfg {
            ignore: vec![s(".git"), s(".repo")],
            endings : vec![s(".c"), s(".cpp"), s(".h")],
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
            endings : vec![s(".c"), s(".cpp"), s(".h")],
        };
        assert!(expected.ignore != cfg.ignore);
    }
    #[test]
    fn test_glob() {
        assert!(Pattern::new("build_*").unwrap().matches("build_Debug"));
    }
    #[test]
    fn test_is_unwanted() {
        let cfg = EnforcerCfg { ignore: vec![s("build_*"), s(".git")], endings: vec![]};
        assert!(is_unwanted(Normal(OsStr::new("build_")), &cfg.ignore));
        assert!(is_unwanted(Normal(OsStr::new("build_Debug")), &cfg.ignore));
        assert!(is_unwanted(Normal(OsStr::new(".git")), &cfg.ignore));
        assert!(!is_unwanted(Normal(OsStr::new("bla")), &cfg.ignore));
    }
}

