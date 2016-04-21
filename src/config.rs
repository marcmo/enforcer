use std;
use std::io;
use toml;
use rustc_serialize::Decodable;
use std::fs;
use std::io::Read;

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct EnforcerCfg {
    pub ignore: Vec<String>,
    pub endings: Vec<String>,
}

pub fn s(x: &str) -> String { x.to_string() }

pub fn get_cfg() -> EnforcerCfg {
    fn read_enforcer_config() -> std::io::Result<EnforcerCfg> {
        let mut cfg_file = try!(fs::File::open(".enforcer"));
        let mut buffer = String::new();
        try!(cfg_file.read_to_string(&mut buffer));
        parse_config(&buffer[..])
    }
    read_enforcer_config().unwrap_or(default_cfg())
}

fn default_cfg() -> EnforcerCfg {
    EnforcerCfg {
        ignore: vec![s(".git"), s(".bake"), s(".repo")],
        endings : vec![s(".c"), s(".cpp"), s(".h")],
    }
}

fn parse_config<'a>(input: &'a str) -> io::Result<EnforcerCfg> {
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
    use super::s;
    use super::EnforcerCfg;
    use super::parse_config;
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
}

