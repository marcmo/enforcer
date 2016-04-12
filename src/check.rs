use std;
use std::io;
use std::path::Path;
use std::fs::File;
use std::fs::metadata;
use std::io::prelude::*;
use clean;

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

pub fn check_path(path: &Path, buf: &[u8], clean: bool, verbose: bool, s: clean::TabStrategy) -> io::Result<u8> {
    let mut check = 0;
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

#[cfg(test)]
mod tests {
    use super::check_content;
    use super::TRAILING_SPACES;
    use super::HAS_TABS;
    use super::HAS_ILLEGAL_CHARACTERS;
    use clean::TabStrategy::Untabify;
    use clean::TabStrategy::Tabify;
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
}

