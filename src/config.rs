use std;
use std::io;
use toml;
use rustc_serialize::Decodable;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct EnforcerCfg {
    pub ignore: Vec<String>,
    pub endings: Vec<String>,
}

pub fn s(x: &str) -> String {
    x.to_string()
}

pub fn get_cfg(config_file: &Path) -> EnforcerCfg {
    let read_enforcer_config = |cnfg| -> std::io::Result<EnforcerCfg> {
        let mut cfg_file = try!(fs::File::open(cnfg));
        let mut buffer = String::new();
        try!(cfg_file.read_to_string(&mut buffer));
        parse_config(&buffer[..])
    };
    match read_enforcer_config(config_file) {
        Ok(c) => c,
        Err(e) => {
            println!("could not read provided config: {}", e.to_string());
            default_cfg()
        },
    }
}

fn default_cfg() -> EnforcerCfg {
    EnforcerCfg {
        ignore: vec![s("**/.git"), s("**/.bake"), s("**/.repo")],
        endings: vec![s(".c"), s(".cpp"), s(".h")],
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
        if !toml.contains_key("ignore") {
            panic!(".enforcer file needs a \"ignore\" section ");
        }
        if !toml.contains_key("endings") {
            panic!(".enforcer file needs a \"endings\" section ");
        }
        let mut decoder = toml::Decoder::new(toml::Value::Table(toml));
        EnforcerCfg::decode(&mut decoder)
            .ok()
            .map_or(Err(default_err()), |config| Ok(config))
    })
}

#[cfg(test)]
mod tests {
    use super::s;
    use super::EnforcerCfg;
    use super::parse_config;

    #[test]
    fn test_load_simple_config() {
        let c = include_str!("../samples/.enforcer");
        let cfg = parse_config(c).unwrap();
        assert_eq!(cfg.ignore.len(), 2);
        let expected = EnforcerCfg {
            ignore: vec![s("**/.git"), s("**/.repo")],
            endings: vec![s(".c"), s(".cpp"), s(".h")],
        };
        assert_eq!(expected.ignore, cfg.ignore);
        assert_eq!(expected, cfg);
    }
    #[test]
    #[should_panic]
    fn test_load_broken_config() {
        let c = include_str!("../samples/.enforcer_broken");
        parse_config(c).unwrap();
    }
}
