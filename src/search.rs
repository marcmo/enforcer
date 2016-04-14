extern crate walkdir;

use self::walkdir::{DirEntry, WalkDir, WalkDirIterator};
use std::path;

pub fn find_matches(start_dir: &path::Path, cfg_ignores: &Vec<String>, file_endings: &Vec<String>) -> Vec<path::PathBuf> {
    let walker = WalkDir::new(start_dir).into_iter();

    let is_hidden = |entry: &DirEntry| -> bool {
        entry.path().components().any (|path_comp| {
            cfg_ignores.iter().any(|to_ignore| Some(path_comp) == path::Path::new(to_ignore).components().next())
        })
    };
    let it = walker.filter_entry(|e| !is_hidden(e))
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
    use super::find_matches;
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
        println!("{:?}", ms);
        assert_eq!(ms.len(), 1);
        assert!(ms.contains(&path::PathBuf::from("./test/matching/test0.cpp")));
    }
}
