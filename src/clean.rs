use std::str::Chars;

#[derive(Clone)]
pub enum TabStrategy { Untabify, Tabify }

fn to_spaces(line: Chars, width: u8) -> String {
    let mut result: Vec<char> = Vec::new();
    let mut column: u8 = 0;
    for c in line {
        match c {
            '\t' => {
                let spaces = width - column;
                for _ in (0..spaces) {result.push(' ')};
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

pub fn tabs_to_spaces<S>(content: S, s: TabStrategy) -> String where S: Into<String>{
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

#[cfg(test)]
mod tests {
    use super::tabs_to_spaces;
    use super::to_spaces;
    use super::TabStrategy;

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
        let c = include_str!("../samples/mixedTabsAndSpaces.cpp");
        let expected = include_str!("../samples/corrected/onlySpaces.cpp");
        let cleaned = tabs_to_spaces(c, TabStrategy::Untabify);
        assert!(cleaned.eq(expected));
    }

    #[test]
    fn test_tabs_to_spaces_with_mixed_in_one_line() {
        let c = include_str!("../samples/mixedTabsAndSpaces2.cpp");
        let expected = include_str!("../samples/corrected/onlySpaces.cpp");
        let cleaned = tabs_to_spaces(c, TabStrategy::Untabify);
        assert!(cleaned.eq(expected));
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
