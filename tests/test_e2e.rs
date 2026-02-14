use std::path::Path;
use std::process::Command;

fn mutator_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    // test binary is in target/debug/deps/, mutator binary is in target/debug/
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("mutator");
    path
}

fn create_python_project(dir: &Path) {
    std::fs::write(
        dir.join("app.py"),
        r#"
def add(a, b):
    return a + b

def is_positive(n):
    return n > 0

def greet(name):
    if name:
        return "hello " + name
    return "hello stranger"
"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("test_app.py"),
        r#"
from app import add, is_positive, greet

def test_add():
    assert add(1, 2) == 3
    assert add(0, 0) == 0
    assert add(-1, 1) == 0

def test_is_positive():
    assert is_positive(1) is True
    assert is_positive(0) is False
    assert is_positive(-1) is False

def test_greet():
    assert greet("world") == "hello world"
    assert greet("") == "hello stranger"
    assert greet(None) == "hello stranger"
"#,
    )
    .unwrap();

    std::fs::write(dir.join("pyproject.toml"), "[project]\nname = \"test-app\"\n").unwrap();
}

#[test]
fn e2e_full_run_json_output() {
    let dir = tempfile::TempDir::new().unwrap();
    create_python_project(dir.path());

    let output = Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "test_app.py", "--json", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nstdout: {stdout}\nstderr: {}", String::from_utf8_lossy(&output.stderr)));

    assert!(result["total"].as_u64().unwrap() > 0, "Should find mutations");
    assert!(result["killed"].as_u64().unwrap() > 0, "Should kill some mutants");
    assert!(result["score"].as_f64().unwrap() > 0.0, "Score should be > 0");
    assert!(result["survived_mutants"].is_array());
}

#[test]
fn e2e_function_scoping() {
    let dir = tempfile::TempDir::new().unwrap();
    create_python_project(dir.path());

    let scoped = Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "test_app.py", "--json", "-f", "add", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    let full = Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "test_app.py", "--json", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    let scoped_result: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&scoped.stdout).trim()).unwrap();
    let full_result: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&full.stdout).trim()).unwrap();

    assert!(
        scoped_result["total"].as_u64().unwrap() < full_result["total"].as_u64().unwrap(),
        "Scoped run should find fewer mutations than full run"
    );
}

#[test]
fn e2e_state_file_written() {
    let dir = tempfile::TempDir::new().unwrap();
    create_python_project(dir.path());

    Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "test_app.py", "--json", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    let state_file = dir.path().join(".mutator-state.json");
    assert!(state_file.exists(), ".mutator-state.json should be written after a run");

    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&state_file).unwrap()).unwrap();
    assert!(state["total"].as_u64().unwrap() > 0);
}

#[test]
fn e2e_status_after_run() {
    let dir = tempfile::TempDir::new().unwrap();
    create_python_project(dir.path());

    Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "test_app.py", "--json", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    let status = Command::new(mutator_bin())
        .args(["status", "--json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator status");

    assert!(status.status.success());
    let result: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&status.stdout).trim()).unwrap();
    assert!(result["total"].as_u64().unwrap() > 0);
}

#[test]
fn e2e_missing_source_file() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("test_app.py"), "").unwrap();

    let output = Command::new(mutator_bin())
        .args(["run", "nonexistent.py", "-t", "test_app.py", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn e2e_missing_test_file() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("app.py"), "x = 1\n").unwrap();

    let output = Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "nonexistent_test.py", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn e2e_invalid_function_name() {
    let dir = tempfile::TempDir::new().unwrap();
    create_python_project(dir.path());

    let output = Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "test_app.py", "-f", "nonexistent_func", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"), "Should report function not found: {stderr}");
}

#[test]
fn e2e_isolation_does_not_modify_original() {
    let dir = tempfile::TempDir::new().unwrap();
    create_python_project(dir.path());

    let original = std::fs::read_to_string(dir.path().join("app.py")).unwrap();

    Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "test_app.py", "--json", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    let after = std::fs::read_to_string(dir.path().join("app.py")).unwrap();
    assert_eq!(original, after, "Source file should not be modified after isolated run");
}

#[test]
fn e2e_quiet_mode_no_output() {
    let dir = tempfile::TempDir::new().unwrap();
    create_python_project(dir.path());

    let output = Command::new(mutator_bin())
        .args(["run", "app.py", "-t", "test_app.py", "-q", "--test-cmd", "pytest"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim().is_empty(), "Quiet mode should produce no stdout, got: {stdout}");
}

#[test]
fn e2e_temp_dirs_cleaned_up() {
    let dir = tempfile::TempDir::new().unwrap();
    create_python_project(dir.path());

    let session_id = "cleanup-test-session";
    Command::new(mutator_bin())
        .args([
            "run", "app.py", "-t", "test_app.py", "--json",
            "--test-cmd", "pytest", "--session", session_id,
        ])
        .current_dir(dir.path())
        .output()
        .expect("failed to run mutator");

    let temp_dir = std::env::temp_dir();
    let leftover: Vec<_> = std::fs::read_dir(&temp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .contains(&format!("mutator-{}", session_id))
        })
        .collect();

    assert!(
        leftover.is_empty(),
        "Temp dirs should be cleaned up after run, found: {:?}",
        leftover.iter().map(|e| e.path()).collect::<Vec<_>>()
    );
}
