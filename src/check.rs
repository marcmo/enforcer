use std;
use std::io;
use std::path::Path;
use std::fs::File;
use std::fs::metadata;
use std::io::prelude::*;
use ansi_term::Colour;
use std::sync::mpsc::{SyncSender};
use clean;

pub const HAS_TABS: u8               = 1 << 0;
pub const TRAILING_SPACES: u8        = 1 << 1;
pub const HAS_ILLEGAL_CHARACTERS: u8 = 1 << 2;
pub const LINE_TOO_LONG: u8          = 1 << 3;

fn check_content<'a>(input: &'a str, filename: &str, verbose: bool, max_line_length: Option<usize>,
                     s: clean::TabStrategy, logger: SyncSender<Option<String>>)
        -> io::Result<u8> {
    debug!("check content of {}", filename);
    let mut result = 0;
    let mut i: u32 = 0;
    for line in input.lines() {
        i += 1;
        if max_line_length.is_some() && line.len() > max_line_length.unwrap() {
            result |= LINE_TOO_LONG;
        }
        if line.ends_with(' ') || line.ends_with('\t') {
            result |= TRAILING_SPACES;
        }
        if s == clean::TabStrategy::Untabify && line.contains("\t") {
            result |= HAS_TABS;
        }
        if line.as_bytes().iter().any(|x| *x > 127) {
            if verbose { let _ = logger.send(Some(format!("non ASCII line [{}]: {} [{}]", i, line, filename))); }
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

fn report_offending_line(path: &Path, logger: SyncSender<Option<String>>) -> std::io::Result<()> {
    use std::io::BufReader;
    let mut i: u32 = 1;
    let f = try!(File::open(path));
    let file = BufReader::new(f);
    for line in file.lines() {
        match line.ok() {
            Some(_) => i = i + 1,
            None => { let _ = logger.send(Some(format!("offending line {} in file [{}]\n", i, path.display()))); },
        }
    }
    Ok(())
}

pub fn check_path(path: &Path, buf: &[u8], clean: bool, verbose: bool,
                  max_line_length: Option<usize>, s: clean::TabStrategy, logger: SyncSender<Option<String>>)
    -> io::Result<u8> {
    let mut check = 0;
    match std::str::from_utf8(buf) {
        Err(_) => {
            check = check | HAS_ILLEGAL_CHARACTERS;
            if verbose {let _ = report_offending_line(path, logger);}
            return Ok(check)
        },
        Ok(buffer) => {
            if check == 0 {
                check = try!(check_content(&buffer, path.to_str().expect("not available"), verbose, max_line_length, s, logger.clone()));
            }
            if clean {
                let no_trailing_ws = if (check & TRAILING_SPACES) > 0 {
                    if verbose { let _ = logger.send(Some(format!("TRAILING_SPACES:[{}] -> removing\n", path.display()))); }
                    clean::remove_trailing_whitespaces(buffer)
                } else { buffer.to_string() };
                let res_string = if (check & HAS_TABS) > 0 {
                    if verbose { let _ = logger.send(Some(format!("HAS_TABS:[{}] -> converting to spaces\n", path.display()))); }
                    clean::space_tabs_conversion(no_trailing_ws, clean::TabStrategy::Untabify)
                } else { no_trailing_ws };
                let mut file = try!(File::create(path));
                try!(file.write_all(res_string.as_bytes()));
            }
            else /* report only */ { if verbose {report(check, &path, logger)} }
        },
    };
    // only check content if we could read the file
    Ok(check)
}

fn report(check: u8, path: &Path, logger: SyncSender<Option<String>>) -> () {
    if check > 0 {
        let mut output = "".to_string();
        if (check & HAS_TABS) > 0 {
            output = output + &format!(":{}", Colour::Red.bold().paint("HAS_TABS"));
        }
        if (check & TRAILING_SPACES) > 0 {
            output = output + &format!(":{}", Colour::Red.bold().paint("TRAILING_SPACES"));
        }
        if (check & HAS_ILLEGAL_CHARACTERS) > 0 {
            output = output + &format!(":{}", Colour::Red.bold().paint("ILLEGAL_CHARACTERS"));
        }
        if (check & LINE_TOO_LONG) > 0 {
            output = output + &format!(":{}", Colour::Yellow.bold().paint("LINE_TOO_LONG"));
        }
        let _ = logger.send(Some(format!("{}:[{}]\n", output, path.display())));
    }
}

#[cfg(test)]
mod tests {
    use super::check_content;
    use super::TRAILING_SPACES;
    use super::HAS_TABS;
    use super::HAS_ILLEGAL_CHARACTERS;
    use super::LINE_TOO_LONG;
    use clean::TabStrategy::Untabify;
    use clean::TabStrategy::Tabify;
    use std::sync::mpsc::{sync_channel};

    #[test]
    fn test_check_good_content() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = " 1\n";
        let res = check_content(content, "foo.h", false, None, Untabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_good_content_with_tabs() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "\t1\n";
        let res = check_content(content, "foo.h", false, None, Tabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_bad_content_with_tabs() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "\t1\n";
        let res = check_content(content, "foo.h", false, None, Untabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 1);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_content_trailing_ws() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "1 \n";
        let res = check_content(content, "foo.h", false, None, Untabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) > 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_content_trailing_tabs() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "1\t\n";
        let res = check_content(content, "foo.h", false, None, Untabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) > 0);
        assert!((check & HAS_TABS) > 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_line_too_long() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "1234567890\n";
        let res = check_content(content, "foo.h", false, Some(5), Tabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
        assert!((check & LINE_TOO_LONG) > 0);
    }
    #[test]
    fn test_line_not_too_long() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "1234567890\n";
        let res = check_content(content, "foo.h", false, Some(10), Tabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
        assert!((check & LINE_TOO_LONG) == 0);
    }
}

