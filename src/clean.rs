use std::str::Chars;

#[derive(Clone, PartialEq)]
pub enum TabStrategy { Untabify, Tabify }

fn to_spaces(line: Chars, width: u8) -> String {
    let mut result: Vec<char> = Vec::new();
    let mut column: u8 = 0;
    for c in line {
        match c {
            '\t' => {
                let spaces = width - column;
                for _ in 0..spaces {result.push(' ')};
                column = 0;
            },
            _ => {
                column = if column == width - 1 { 0 } else { (column+1)%width };
                result.push(c)
            },
        }
    }
    result.into_iter().collect()
}

fn to_tabs(_: Chars, _: u8) -> String {
    String::new()
}

pub fn space_tabs_conversion<S>(content: S, s: TabStrategy) -> String where S: Into<String>{
    let converted: Vec<String> = content.into().lines()
        .map(|line| {
            match s {
                TabStrategy::Untabify => to_spaces(line.chars(), 4),
                TabStrategy::Tabify => to_tabs(line.chars(), 4),
            }
        }).collect();
    let mut res = converted.join("\n");
    res.push_str("\n");
    res
}

pub fn remove_trailing_whitespaces<S>(input: S) -> String where S: Into<String>{
    let s = input.into();
    let v: Vec<&str> = s
        .lines()
        .map(|line| line.trim_right())
        .collect();

    if s.ends_with("\n") {
        v.join("\n") + "\n"
    }
    else {
        v.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::remove_trailing_whitespaces;
    use super::space_tabs_conversion;
    use super::to_spaces;
    use super::TabStrategy;

    #[test]
    fn test_clean_does_not_remove_trailing_newline() {
        let content = "1\n2\n3\n4\n5\n";
        let cleaned = remove_trailing_whitespaces(content);
        assert!(cleaned.eq(content));
    }
    #[test]
    fn test_clean_trailing_whitespace() {
        let content = "1 \n2";
        let cleaned = remove_trailing_whitespaces(content);
        println!("{:?}", cleaned);
        assert!(cleaned.eq("1\n2"));
    }
    #[test]
    fn test_clean_trailing_tabs() {
        let content = "1\t\n2";
        let cleaned = remove_trailing_whitespaces(content);
        println!("{:?}", cleaned);
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
    fn test_file_with_tabs_to_spaces() {
        let c = include_str!("../samples/mixedTabsAndSpaces.cpp");
        let expected = include_str!("../samples/corrected/mixedTabsAndSpaces.cpp");
        let cleaned = space_tabs_conversion(c, TabStrategy::Untabify);
        assert_eq!(cleaned, expected);
    }
    #[test]
    fn test_file_with_tabs_and_trailing_whitespaces() {
        let c = include_str!("../samples/withTabsAndTrailingWhitespaces.cpp");
        let expected = include_str!("../samples/corrected/withTabsAndTrailingWhitespaces.cpp");
        let cleaned = remove_trailing_whitespaces(space_tabs_conversion(c, TabStrategy::Untabify));
        assert_eq!(cleaned, expected);
        let cleaned2 = space_tabs_conversion(remove_trailing_whitespaces(c), TabStrategy::Untabify);
        assert_eq!(cleaned2, expected);
    }

    #[test]
    fn test_file_with_tabs_and_spaces_to_spaces() {
        let c = include_str!("../samples/mixedTabsAndSpaces2.cpp");
        let expected = include_str!("../samples/corrected/mixedTabsAndSpaces.cpp");
        let cleaned = space_tabs_conversion(c, TabStrategy::Untabify);
        assert_eq!(cleaned, expected);
    }
    #[test]
    fn test_file_with_empty_line() {
        let c = include_str!("../samples/simpleWithEmptyLine.cpp");
        let expected = include_str!("../samples/corrected/simpleWithEmptyLine.cpp");
        let cleaned = space_tabs_conversion(c, TabStrategy::Untabify);
        assert_eq!(cleaned, expected);
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
