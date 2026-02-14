use mutator::safety;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn backup_path_format() {
    let path = safety::backup_path(Path::new("/tmp/foo.py"));
    assert_eq!(path, Path::new("/tmp/.foo.py.mutator.bak"));
}

#[test]
fn backup_path_nested() {
    let path = safety::backup_path(Path::new("/home/user/project/src/app.py"));
    assert_eq!(path, Path::new("/home/user/project/src/.app.py.mutator.bak"));
}

#[test]
fn check_interrupted_run_returns_none_when_clean() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("test.py");
    std::fs::write(&source, "pass").unwrap();
    assert!(safety::check_interrupted_run(&source).is_none());
}

#[test]
fn check_interrupted_run_returns_path_when_backup_exists() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("test.py");
    let backup = dir.path().join(".test.py.mutator.bak");
    std::fs::write(&source, "mutated").unwrap();
    std::fs::write(&backup, "original").unwrap();
    let result = safety::check_interrupted_run(&source);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), backup);
}

#[test]
fn restore_from_backup_restores_and_cleans() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("test.py");
    let backup = dir.path().join(".test.py.mutator.bak");

    std::fs::write(&source, "mutated").unwrap();
    std::fs::write(&backup, "original").unwrap();

    safety::restore_from_backup(&source, &backup).unwrap();
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "original");
    assert!(!backup.exists());
}
