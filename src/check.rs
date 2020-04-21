use ansi_term;
use std;
use std::fs::metadata;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use unic_char_range::CharRange;

#[cfg(not(target_os = "windows"))]
use term_painter::{Attr::*, Color, Painted, ToStyle};

use crate::clean;
use std::sync::mpsc::SyncSender;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InfoLevel {
    Quiet,
    Normal,
    Verbose,
}

static UTF8_ALLOWED_RANGES: &[CharRange] = &[
    // Basic Latin
    CharRange {
        low: '\u{0000}',
        high: '\u{007f}',
    },
    // Latin supplement -- without control characters and no-break space
    CharRange {
        low: '\u{00a1}',
        high: '\u{00ff}',
    },
    // Latin extended A
    CharRange {
        low: '\u{0100}',
        high: '\u{017f}',
    },
    // Latin extended B
    CharRange {
        low: '\u{0180}',
        high: '\u{024f}',
    },
    // Box drawing
    CharRange {
        low: '\u{2500}',
        high: '\u{257f}',
    },
    // Block elements
    CharRange {
        low: '\u{2580}',
        high: '\u{259f}',
    },
    // Geometric shapes
    CharRange {
        low: '\u{25a0}',
        high: '\u{25ff}',
    },
];

pub const HAS_TABS: u8 = 1;
pub const TRAILING_SPACES: u8 = 1 << 1;
pub const HAS_ILLEGAL_CHARACTERS: u8 = 1 << 2;
pub const LINE_TOO_LONG: u8 = 1 << 3;
pub const HAS_WINDOWS_LINE_ENDINGS: u8 = 1 << 4;

fn check_content<'a>(
    input: &'a str,
    filename: &str,
    info_level: InfoLevel,
    max_line_length: Option<usize>,
    s: clean::TabStrategy,
    logger: SyncSender<Option<String>>,
) -> io::Result<u8> {
    let mut result = 0;
    let mut i: u32 = 0;
    for line in input.lines() {
        i += 1;

        if let Some(max_len) = max_line_length {
            if line.len() > max_len {
                result |= LINE_TOO_LONG;
                if info_level == InfoLevel::Verbose {
                    let _ = logger.send(Some(format!(
                        "{}, line {}: error: LINE_TOO_LONG\n",
                        filename, i
                    )));
                }
            }
        }
        if line.ends_with(' ') || line.ends_with('\t') {
            result |= TRAILING_SPACES;
            if info_level == InfoLevel::Verbose {
                let _ = logger.send(Some(format!(
                    "{}, line {}: error: TRAILING_SPACES\n",
                    filename, i
                )));
            }
        }
        if s == clean::TabStrategy::Untabify && line.contains('\t') {
            result |= HAS_TABS;
            if info_level == InfoLevel::Verbose {
                let _ = logger.send(Some(format!("{}, line {}: error: HAS_TABS\n", filename, i)));
            }
        }
        if !line
            .chars()
            .all(|c| UTF8_ALLOWED_RANGES.iter().any(|range| range.contains(c)))
        {
            result |= HAS_ILLEGAL_CHARACTERS;
            if info_level == InfoLevel::Verbose {
                let _ = logger.send(Some(format!(
                    "{}, line {}: error: non ASCII line\n",
                    filename, i
                )));
            }
        }
    }
    if input.contains("\r\n") {
        result |= HAS_WINDOWS_LINE_ENDINGS;
        if info_level == InfoLevel::Verbose {
            let _ = logger.send(Some(format!(
                "{}: error: HAS_WINDOWS_LINE_ENDINGS\n",
                filename
            )));
        }
    }
    if info_level == InfoLevel::Normal {
        if (result & LINE_TOO_LONG) > 0 {
            let _ = logger.send(Some(format!(
                "{}, some lines with LINE_TOO_LONG\n",
                filename
            )));
        }
        if (result & TRAILING_SPACES) > 0 {
            let _ = logger.send(Some(format!(
                "{}, some lines with TRAILING_SPACES\n",
                filename
            )));
        }
        if (result & HAS_TABS) > 0 {
            let _ = logger.send(Some(format!("{}, some lines with HAS_TABS\n", filename)));
        }
        if (result & HAS_WINDOWS_LINE_ENDINGS) > 0 {
            let _ = logger.send(Some(format!(
                "{}, some lines with HAS_WINDOWS_LINE_ENDINGS\n",
                filename
            )));
        }
        if (result & HAS_ILLEGAL_CHARACTERS) > 0 {
            let _ = logger.send(Some(format!(
                "{}, some lines with non ASCII line\n",
                filename
            )));
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
    let f = File::open(path)?;
    let file = BufReader::new(f);
    for line in file.lines() {
        match line.ok() {
            Some(_) => i += 1,
            None => {
                let _ = logger.send(Some(format!(
                    "{}, line {}: error: non UTF-8 character in line\n",
                    path.display(),
                    i
                )));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn check_path(
    path: &Path,
    buf: &[u8],
    clean: bool,
    info_level: InfoLevel,
    max_line_length: Option<usize>,
    s: clean::TabStrategy,
    ending: clean::LineEnding,
    logger: SyncSender<Option<String>>,
) -> io::Result<u8> {
    let mut check = 0;
    match std::str::from_utf8(buf) {
        Err(_) => {
            check |= HAS_ILLEGAL_CHARACTERS;
            if info_level == InfoLevel::Normal {
                let _ = report_offending_line(path, logger);
            }
            return Ok(check);
        }
        Ok(buffer) => {
            if check == 0 {
                check = check_content(
                    &buffer,
                    path.to_str().expect("not available"),
                    info_level,
                    max_line_length,
                    s,
                    logger.clone(),
                )?;
            }
            let no_trailing_ws = if (check & TRAILING_SPACES) > 0 && clean {
                if info_level == InfoLevel::Verbose {
                    let _ = logger.send(Some(format!(
                        "TRAILING_SPACES:[{}] -> removing\n",
                        path.display()
                    )));
                }
                clean::remove_trailing_whitespaces(buffer, &ending)
            } else {
                buffer.to_string()
            };
            let space_tab_converted = if (check & HAS_TABS) > 0 && clean {
                if info_level == InfoLevel::Verbose {
                    let _ = logger.send(Some(format!(
                        "HAS_TABS:[{}] -> converting to spaces\n",
                        path.display()
                    )));
                }
                clean::space_tabs_conversion(no_trailing_ws, clean::TabStrategy::Untabify, ending)
            } else {
                no_trailing_ws
            };
            let res_string = if (check & HAS_WINDOWS_LINE_ENDINGS) > 0 && clean {
                if info_level == InfoLevel::Verbose {
                    let _ = logger.send(Some(format!(
                        "HAS_WINDOWS_LINE_ENDINGS:[{}] -> converting CRLF to LF\n",
                        path.display()
                    )));
                }
                clean::replace_win_line_endings(space_tab_converted)
            } else {
                space_tab_converted
            };
            if clean {
                let mut file = File::create(path)?;
                file.write_all(res_string.as_bytes())?;
            }
        }
    };
    // only check content if we could read the file
    Ok(check)
}

#[allow(dead_code)]
#[cfg(not(target_os = "windows"))]
pub fn red(s: &str) -> ansi_term::ANSIString {
    ansi_term::Colour::Red.bold().paint(s)
}
#[allow(dead_code)]
#[cfg(not(target_os = "windows"))]
pub fn yellow(s: &str) -> ansi_term::ANSIString {
    ansi_term::Colour::Yellow.bold().paint(s)
}
#[cfg(not(target_os = "windows"))]
pub fn green(s: &str) -> Painted<&str> {
    Color::Green.paint(s)
}
#[cfg(not(target_os = "windows"))]
pub fn bold(s: &str) -> Painted<&str> {
    Bold.paint(s)
}
#[cfg(target_os = "windows")]
pub fn green(s: &str) -> ansi_term::ANSIString {
    ansi_term::Style::new().paint(s)
}
#[cfg(target_os = "windows")]
pub fn bold(s: &str) -> ansi_term::ANSIString {
    ansi_term::Style::new().paint(s)
}

#[cfg(test)]
mod tests {
    use super::check_content;
    use super::InfoLevel;
    use super::HAS_ILLEGAL_CHARACTERS;
    use super::HAS_TABS;
    use super::HAS_WINDOWS_LINE_ENDINGS;
    use super::LINE_TOO_LONG;
    use super::TRAILING_SPACES;
    use crate::clean::TabStrategy::{Tabify, Untabify};
    use std::sync::mpsc::sync_channel;

    #[test]
    fn test_check_good_content() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = " 1\n";
        let res = check_content(
            content,
            "foo.h",
            InfoLevel::Quiet,
            None,
            Untabify,
            logging_tx,
        );
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
        let res = check_content(content, "foo.h", InfoLevel::Quiet, None, Tabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_good_content_with_box_drawing() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = r#"
                     ▲
                     │
                     ▼
            ┌────────S────────┐
            │░░░░░░░░░░░░░░░░░│
            │░Internal memory░│
            │░░░░░░░░░░░░░░░░░│
            └─────────────────┘
            "#;

        let res = check_content(content, "foo.h", InfoLevel::Quiet, None, Tabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();

        // There are trailing spaces in the string literal, but otherwise it's good
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_bad_content_with_illegal_characters() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = r#"
            Бл҃же́нъ мꙋ́жъ, и҆́же не и҆́де на совѣ́тъ нечести́выхъ,
            и҆ на пꙋтѝ грѣ́шныхъ не ста̀, и҆ на сѣда́лищи гꙋби́телей
            не сѣ́де: но въ зако́нѣ гдⷭ҇ни во́лѧ є҆гѡ̀, и҆ въ зако́нѣ
            є҆гѡ̀ поꙋчи́тсѧ де́нь и҆ но́щь."#;

        let res = check_content(content, "foo.h", InfoLevel::Quiet, None, Tabify, logging_tx);
        assert!(res.is_ok());
        let check = res.unwrap();

        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) != 0);
    }
    #[test]
    fn test_check_bad_content_with_tabs() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "\t1\n";
        let res = check_content(
            content,
            "foo.h",
            InfoLevel::Quiet,
            None,
            Untabify,
            logging_tx,
        );
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 1);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_bad_content_with_win_line_endings() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "1\r\n2\r\n";
        let res = check_content(
            content,
            "foo.h",
            InfoLevel::Quiet,
            None,
            Untabify,
            logging_tx,
        );
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_WINDOWS_LINE_ENDINGS) > 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
    }
    #[test]
    fn test_check_content_trailing_ws() {
        let (logging_tx, _) = sync_channel::<Option<String>>(0);
        let content = "1 \n";
        let res = check_content(
            content,
            "foo.h",
            InfoLevel::Quiet,
            None,
            Untabify,
            logging_tx,
        );
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
        let res = check_content(
            content,
            "foo.h",
            InfoLevel::Quiet,
            None,
            Untabify,
            logging_tx,
        );
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
        let res = check_content(
            content,
            "foo.h",
            InfoLevel::Quiet,
            Some(5),
            Tabify,
            logging_tx,
        );
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
        let res = check_content(
            content,
            "foo.h",
            InfoLevel::Quiet,
            Some(10),
            Tabify,
            logging_tx,
        );
        assert!(res.is_ok());
        let check = res.unwrap();
        assert!((check & TRAILING_SPACES) == 0);
        assert!((check & HAS_TABS) == 0);
        assert!((check & HAS_ILLEGAL_CHARACTERS) == 0);
        assert!((check & LINE_TOO_LONG) == 0);
    }
}
