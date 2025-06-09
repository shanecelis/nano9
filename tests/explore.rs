use std::path::{Path, PathBuf};

#[test]
fn test_path_dotdot() {
    let mut p = PathBuf::from("a/b/c");
    p.push("../d");
    assert_eq!(p, Path::new("a/b/c/../d"));
}

#[test]
fn test_path0() {
    let mut p = PathBuf::from("a/b/c/");
    assert!(p.pop());
    p.push("d");
    assert_eq!(p, Path::new("a/b/d"));
    assert!(p.pop());
    assert_eq!(p, Path::new("a/b"));
    assert!(p.pop());
    assert_eq!(p, Path::new("a"));
    assert_eq!(p, Path::new("a/"));
    assert!(p.pop());
    assert_eq!(p, Path::new(""));
    assert_eq!(p, Path::new(""));
    assert!(!p.pop());
}
