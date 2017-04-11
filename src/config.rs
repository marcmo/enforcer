use std;
use std::io;
use toml;
use rustc_serialize::Decodable;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::io::{Error, ErrorKind};
use regex::Regex;

const DEFAULT_CFG_FILE: &'static str = "./.enforcer";

#[derive(Debug, RustcDecodable, PartialEq)]
pub struct EnforcerCfg {
    pub ignore: Vec<String>,
    pub endings: Vec<String>,
}

pub fn s(x: &str) -> String {
    x.to_string()
}

fn load_default_cfg_file() -> io::Result<fs::File> {
    let p = PathBuf::from(DEFAULT_CFG_FILE);
    if !p.as_path().exists() {
        println!("default config file {:?} does not exist!", DEFAULT_CFG_FILE);
        Err(Error::new(ErrorKind::NotFound, "default config file missing"))
    } else {
        fs::File::open(p)
    }
}

pub fn get_cfg(config_file: &Option<PathBuf>) -> EnforcerCfg {
    let read_enforcer_config = |cnfg: &Option<PathBuf>| -> std::io::Result<EnforcerCfg> {
        let mut cfg_file = match *cnfg {
            Some(ref p) => {
                if !p.as_path().exists() {
                    println!("provided file {:?} does not exist!", p);
                }
                fs::File::open(p)?
            }
            None => load_default_cfg_file()?,
        };
        let mut buffer = String::new();
        cfg_file.read_to_string(&mut buffer)?;
        parse_config(&buffer[..])
    };
    match read_enforcer_config(config_file) {
        Ok(c) => c,
        _ => {
            println!("taking default configuration: {:?}", default_cfg());
            default_cfg()
        }
    }
}

fn default_cfg() -> EnforcerCfg {
    EnforcerCfg {
        ignore: vec![s("**/.git"), s("**/.bake"), s("**/.repo")],
        endings: vec![s(".c"), s(".cpp"), s(".h")],
    }
}

fn fix_config(cfg: &EnforcerCfg) -> EnforcerCfg {
    EnforcerCfg {
        ignore: cfg.ignore.iter().map(|i| suggestion(i)).collect::<Vec<String>>(),
        endings: cfg.endings.clone(),
    }
}

fn full_match(r: &Regex, s: &str) -> bool {
    if let Some(mat) = r.find(s) {
        mat.start() == 0 && mat.end() == s.len()
    } else {
        false
    }
}
fn suggestion(s: &str) -> String {
    let full_component = Regex::new(r"[a-zA-Z_\-\d]+\*?").expect("valid regex");
    let ending = Regex::new(r"(\*?|[a-zA-Z_\-\d]+)\.[a-zA-Z_\-\d]+\*?").expect("valid regex");
    if full_match(&full_component, s) {
        String::from("**/") + s + "/**"
    } else if full_match(&ending, s) {
        String::from("**/") + s
    } else {
        s.to_string()
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
            .map_or(Err(default_err()), |config| {
                let suggested = fix_config(&config);
                if suggested.ignore != config.ignore {
                    println!("old style config found. we will assume this:\n{:?}\nconsider \
                              changing it! (see http://www.globtester.com/ for reference)",
                             suggested);
                    Ok(suggested)
                } else {
                    Ok(config)
                }
            })
    })
}

#[cfg(test)]
mod tests {
    use super::s;
    use super::EnforcerCfg;
    use super::parse_config;
    use super::suggestion;

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

    #[test]
    fn test_matches() {
        assert_eq!("**/abc/**".to_string(), suggestion("abc"));
        assert_eq!("**/.repo".to_string(), suggestion(".repo"));
        assert_eq!("**/.repo2".to_string(), suggestion(".repo2"));
        assert_eq!("**/build_*/**".to_string(), suggestion("build_*"));
        assert_eq!("**/*.o".to_string(), suggestion("*.o"));
        assert_eq!("**/*.dld".to_string(), suggestion("*.dld"));
        assert_eq!("**/*.s".to_string(), suggestion("*.s"));
        assert_eq!("**/autosarOs/**".to_string(), suggestion("autosarOs"));
        assert_eq!("**/fat32/**".to_string(), suggestion("fat32"));
        assert_eq!("**/gtest.h".to_string(), suggestion("gtest.h"));
    }
}
