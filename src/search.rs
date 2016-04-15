extern crate walkdir;
extern crate regex;

use self::walkdir::{DirEntry, WalkDir, WalkDirIterator};
use std::path;
use self::regex::Regex;

// find out if any path component in the path fully matches the regex
fn path_components_matches(to_ignore: &regex::Regex, path: &path::Path) -> bool {
    path.components().any (|path_comp| {
        match path_comp.as_os_str().to_str(){
            Some(s) => {
                match to_ignore.find(s) {
                    Some((0,b)) => { s.len() == b },
                    _ => false
                }
            }
            None => false
        }
    })
}

pub fn find_matches(start_dir: &path::Path, cfg_ignores: &Vec<String>, file_endings: &Vec<String>) -> Vec<path::PathBuf> {
    let walker = WalkDir::new(start_dir).into_iter();
    let ignore_regex = cfg_ignores.iter().fold(Vec::new(), |mut acc, ignore_glob| {
        let r = ignore_glob.replace("*",".*").replace("?",".");
        acc.push(Regex::new(&r).unwrap());
        acc
    });

    let to_ignore = |entry: &DirEntry| -> bool {
        ignore_regex.iter().any(|to_ignore| {
            path_components_matches(to_ignore, entry.path())
        })
    };
    let it = walker.filter_entry(|e| !to_ignore(e))
        .filter_map(|e| e.ok())
        .into_iter();
    let mut res = Vec::new();
    for f in it {
        if !f.file_type().is_file() {
            continue;
        }
        if f.file_name().to_str().map(|f|{
            file_endings.iter().any (|p| f.ends_with(p))
        }).unwrap_or(false) {
            res.push(f.path().to_owned());
        }
    }
    res
}

#[cfg(test)]
mod tests {
    extern crate regex;
    use super::find_matches;
    use super::path_components_matches;
    use self::regex::Regex;
    use std::path;

    fn s(x: &str) -> String { x.to_string() }

    #[test]
    fn test_find_all_matches() {
        let ignores = vec![s(".git"), s(".bake"), s(".repo")];
        let endings = vec![s(".cpp")];
        let ms = find_matches(path::Path::new("./test/matching"), &ignores, &endings);
        assert_eq!(ms.len(), 2);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
        assert!(ms.contains(&path::PathBuf::from("./test/matching/abc/test1.cpp")));
    }

    #[test]
    fn test_ignore_some_paths() {
        let ignores = vec![s("abc"), s(".bake"), s(".repo")];
        let endings = vec![s(".cpp")];
        let ms = find_matches(path::Path::new("./test/matching"), &ignores, &endings);
        assert_eq!(ms.len(), 1);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
    }

    #[test]
    fn test_ignore_some_paths_with_globs() {
        let ignores = vec![s("ab*")];
        let endings = vec![s(".cpp")];
        let ms = find_matches(path::Path::new("./test/matching"), &ignores, &endings);
        assert_eq!(ms.len(), 1);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
    }
    #[test]
    fn test_ignore_some_paths_with_globs2() {
        let ignores = vec![s("a?c")];
        let endings = vec![s(".cpp")];
        let ms = find_matches(path::Path::new("./test/matching"), &ignores, &endings);
        assert_eq!(ms.len(), 1);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
    }
    #[test]
    fn test_path_component_matches_full_match() {
        let rx = Regex::new("a.c").unwrap();
        let path = path::Path::new("./test/abc/me.cpp");
        assert!(path_components_matches(&rx, &path));
    }
    #[test]
    fn test_path_component_matches_partial_match() {
        let rx = Regex::new("a.c").unwrap();
        let path = path::Path::new("./test/aabcd/me.cpp");
        assert!(!path_components_matches(&rx, &path));
    }
    #[test]
    fn test_path_component_matches_at_begining() {
        let rx = Regex::new("a.*").unwrap();
        let path = path::Path::new("./test/abc/me.cpp");
        assert!(path_components_matches(&rx, &path));
    }
    #[test]
    fn test_path_component_matches_at_end() {
        let rx = Regex::new(".*bc").unwrap();
        let path = path::Path::new("./test/abc/me.cpp");
        assert!(path_components_matches(&rx, &path));
    }

}
