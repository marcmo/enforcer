extern crate walkdir;
extern crate glob;

use walkdir::{DirEntry, WalkDir, WalkDirIterator};
use std::path;
use glob::Pattern;

// find out if any path component in the path fully matches the pattern
fn path_components_matches(pattern: &str, path: &path::Path) -> bool {
    let haystack = path.to_str().expect(format!("problems with path: {:?}", &path).as_str());
    let cleaned = if haystack.starts_with("./") {
        &haystack[2..]
    } else {
        haystack
    };
    match Pattern::new(pattern) {
        Ok(pat) => pat.matches(cleaned),
        Err(e) => {
            println!("problems with pattern: {:?}({})", pattern, e);
            false
        }
    }
}

pub fn find_matches(start_dir: &path::Path,
                    cfg_ignores: &Vec<String>,
                    file_endings: &[String])
                    -> Vec<path::PathBuf> {
    let walker = WalkDir::new(start_dir).into_iter();
    let to_ignore = |entry: &DirEntry| -> bool {
        cfg_ignores.iter().any(|to_ignore| path_components_matches(to_ignore, entry.path()))
    };
    let it = walker.filter_entry(|e| !to_ignore(e))
        .filter_map(|e| e.ok())
        .into_iter();
    let mut res = Vec::new();
    let endings = file_endings.iter().fold(Vec::new(), |mut acc, ending| {
        // support old way of writing file endings
        acc.push(ending.replace("**/*", ""));
        acc
    });
    for f in it {
        if !f.file_type().is_file() {
            continue;
        }
        if f.file_name()
            .to_str()
            .map(|f| endings.iter().any(|p| f.ends_with(p)))
            .unwrap_or(false) {
            res.push(f.path().to_owned());
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::find_matches;
    use super::path_components_matches;
    use std::path;

    fn s(x: &str) -> String {
        x.to_string()
    }

    #[test]
    fn test_find_all_matches() {
        let ignores = vec![s(".git"), s(".bake"), s("**/secret.cpp")];
        let endings = vec![s(".cpp")];
        let ms = find_matches(path::Path::new("./test/matching"), &ignores, &endings);
        assert_eq!(ms.len(), 2);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
        assert!(ms.contains(&path::PathBuf::from("./test/matching/abc/test1.cpp")));
    }

    #[test]
    fn test_ignore_some_paths() {
        let ignores = vec![s("**/abc/**"), s(".bake"), s("**/secret.cpp")];
        let endings = vec![s(".cpp")];
        let ms = find_matches(path::Path::new("./test/matching"), &ignores, &endings);
        assert_eq!(ms.len(), 1);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
        assert!(!ms.contains(&path::PathBuf::from("./test/matching/secret.cpp")));
    }

    #[test]
    fn test_ignore_some_paths_with_globs() {
        let ignores = vec![s("**/ab*/**")];
        let endings = vec![s(".cpp")];
        let ms = find_matches(path::Path::new("./test/matching"), &ignores, &endings);
        assert_eq!(ms.len(), 2);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
        assert!(ms.contains(&path::PathBuf::from("./test/matching/secret.cpp")));
    }
    #[test]
    fn test_ignore_some_paths_with_globs2() {
        let ignores = vec![s("**/a?c/**")];
        let endings = vec![s(".cpp")];
        let ms = find_matches(path::Path::new("./test/matching"), &ignores, &endings);
        assert_eq!(ms.len(), 2);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
        assert!(ms.contains(&path::PathBuf::from("./test/matching/secret.cpp")));
    }
    #[test]
    fn test_path_component_matches_with_globs() {
        let path = path::Path::new("./test/abc/me.cpp");
        assert!(path_components_matches("**/a?c/**", &path));
    }
    #[test]
    fn test_path_component_matches_multiple_path_elements() {
        let path = path::Path::new("./test/abc/me.cpp");
        assert!(path_components_matches("**/test/abc/**", &path));
    }
    #[test]
    fn test_path_component_matches_full_match() {
        let path = path::Path::new("./test/abc/me.cpp");
        assert!(path_components_matches("**/a?c/**", &path));
    }
    #[test]
    fn test_path_component_matches_partial_match() {
        let path = path::Path::new("./test/aabcd/me.cpp");
        assert!(!path_components_matches("**/a?c/**", &path));
    }
    #[test]
    fn test_path_component_matches_at_begining() {
        let path = path::Path::new("./test/abc/me.cpp");
        assert!(path_components_matches("**/a*/**", &path));
    }
    #[test]
    fn test_path_component_matches_at_end() {
        let path = path::Path::new("./test/abc/me.cpp");
        assert!(path_components_matches("**/*bc/**", &path));
    }

}
