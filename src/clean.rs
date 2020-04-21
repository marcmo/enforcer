use std::str::Chars;

#[derive(Clone, PartialEq)]
pub enum TabStrategy {
    Untabify,
    Tabify,
}
#[derive(Clone, PartialEq)]
pub enum LineEnding {
    LF,
    CRLF,
}

fn to_spaces(line: Chars, width: u8) -> String {
    let mut result: Vec<char> = Vec::new();
    let mut column: u8 = 0;
    for c in line {
        match c {
            '\t' => {
                let spaces = width - column;
                for _ in 0..spaces {
                    result.push(' ')
                }
                column = 0;
            }
            _ => {
                column = if column == width - 1 {
                    0
                } else {
                    (column + 1) % width
                };
                result.push(c)
            }
        }
    }
    result.into_iter().collect()
}

fn to_tabs(_: Chars, _: u8) -> String {
    String::new()
}

pub fn space_tabs_conversion<S>(content: S, s: TabStrategy, line_ending: LineEnding) -> String
where
    S: Into<String>,
{
    let converted: Vec<String> = content
        .into()
        .lines()
        .map(|line| match s {
            TabStrategy::Untabify => to_spaces(line.chars(), 4),
            TabStrategy::Tabify => to_tabs(line.chars(), 4),
        })
        .collect();
    let ending = match line_ending {
        LineEnding::LF => "\n",
        LineEnding::CRLF => "\r\n",
    };
    let mut res = converted.join(ending);
    res.push_str(ending);
    res
}

pub fn remove_trailing_whitespaces<S>(input: S, line_ending: &LineEnding) -> String
where
    S: Into<String>,
{
    let s = input.into();
    let v: Vec<&str> = s.lines().map(|line| line.trim_end()).collect();

    let ending = match line_ending {
        LineEnding::LF => "\n",
        LineEnding::CRLF => "\r\n",
    };
    if s.ends_with('\n') {
        v.join(ending) + ending
    } else {
        v.join(ending)
    }
}

pub fn replace_win_line_endings<S>(input: S) -> String
where
    S: Into<String>,
{
    input.into().replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_does_not_remove_trailing_newline() {
        let content = "1\n2\n3\n4\n5\n";
        let cleaned = remove_trailing_whitespaces(content, &LineEnding::LF);
        assert!(cleaned.eq(content));
    }
    #[test]
    fn test_clean_trailing_whitespace() {
        let content = "1 \n2";
        let cleaned = remove_trailing_whitespaces(content, &LineEnding::LF);
        assert!(cleaned.eq("1\n2"));
    }
    #[test]
    fn test_clean_win_line_endings() {
        let content = "1\r\n2";
        let cleaned = replace_win_line_endings(content);
        assert!(cleaned.eq("1\n2"));
    }
    #[test]
    fn test_clean_trailing_tabs() {
        let content = "1\t\n2";
        let cleaned = remove_trailing_whitespaces(content, &LineEnding::LF);
        assert!(cleaned.eq("1\n2"));
    }
    #[test]
    fn test_convert_line_with_leading_tab() {
        let line = "\t    a".chars();
        let converted = to_spaces(line, 4);
        assert_eq!(converted, "        a");
    }
    #[test]
    fn test_convert_line_with_tab_at_end_of_spaces() {
        let line = "    \tA".chars();
        let converted = to_spaces(line, 4);
        assert_eq!(converted, "        A");
    }
    #[test]
    fn test_tabs_to_spaces() {
        let line1 = "		foo";
        let line2 = "		bar";
        let text_with_tabs_newline = [line1, line2].join("\n");
        let text_with_tabs_cr_ln = [line1, line2].join("\r\n");
        let expected_ln = "        foo\n        bar\n";
        let cleaned_ln = space_tabs_conversion(
            text_with_tabs_newline,
            TabStrategy::Untabify,
            LineEnding::LF,
        );
        assert_eq!(cleaned_ln, expected_ln);
        let expected_cr_ln = "        foo\r\n        bar\r\n";
        let cleaned_cr_ln = space_tabs_conversion(
            text_with_tabs_cr_ln,
            TabStrategy::Untabify,
            LineEnding::CRLF,
        );
        assert_eq!(cleaned_cr_ln, expected_cr_ln);
    }
    #[test]
    fn test_mixed_tabs_and_spaces_to_spaces() {
        let line1 = "		foo";
        let line2 = "       bar";
        let text_with_tabs_newline = [line1, line2].join("\n");
        let text_with_tabs_cr_ln = [line1, line2].join("\r\n");
        let expected_ln = "        foo\n       bar\n";
        let cleaned_ln = space_tabs_conversion(
            text_with_tabs_newline,
            TabStrategy::Untabify,
            LineEnding::LF,
        );
        assert_eq!(cleaned_ln, expected_ln);
        let expected_cr_ln = "        foo\r\n       bar\r\n";
        let cleaned_cr_ln = space_tabs_conversion(
            text_with_tabs_cr_ln,
            TabStrategy::Untabify,
            LineEnding::CRLF,
        );
        assert_eq!(cleaned_cr_ln, expected_cr_ln);
    }

    #[test]
    fn test_tabs_and_spaces_in_one_line() {
        let line1 = " 		foo"; // space + tab + tab
        let line2 = "		bar"; // tab + tab
        let text_with_tabs_newline = [line1, line2].join("\n");
        let text_with_tabs_cr_ln = [line1, line2].join("\r\n");
        let expected_ln = "        foo\n        bar\n";
        let cleaned_ln = space_tabs_conversion(
            text_with_tabs_newline,
            TabStrategy::Untabify,
            LineEnding::LF,
        );
        assert_eq!(cleaned_ln, expected_ln);
        let expected_cr_ln = "        foo\r\n        bar\r\n";
        let cleaned_cr_ln = space_tabs_conversion(
            text_with_tabs_cr_ln,
            TabStrategy::Untabify,
            LineEnding::CRLF,
        );
        assert_eq!(cleaned_cr_ln, expected_cr_ln);
    }
    #[test]
    fn test_no_change_on_empty_string() {
        let line = "".chars();
        let converted = to_spaces(line, 4);
        assert_eq!(converted, "");
    }
    #[test]
    fn test_tabs_at_end() {
        let line = "foobarbaz\t".chars();
        let converted = to_spaces(line, 4);
        assert_eq!(converted, "foobarbaz   ");
    }
    #[test]
    fn test_tabs_in_middle() {
        let line = "foo\tbar\tbaz".chars();
        let converted = to_spaces(line, 4);
        assert_eq!(converted, "foo bar baz");
    }
    #[test]
    fn test_one_tab_to_spaces() {
        let line = "\tfoo".chars();
        let converted = to_spaces(line, 2);
        assert_eq!(converted, "  foo");
    }
    #[test]
    fn test_two_tabs_to_spaces() {
        let line = "\t\tfoo".chars();
        let converted = to_spaces(line, 2);
        assert_eq!(converted, "    foo");
    }
    #[test]
    fn test_change_mixed_tabs_and_spaces() {
        let line = " \t foo".chars();
        let converted = to_spaces(line, 2);
        assert_eq!(converted, "   foo");
    }
}
