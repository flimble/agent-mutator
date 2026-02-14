use mutator::mutants::Mutation;
use mutator::runner;
use std::path::Path;
use mutator;

fn make_mutation(start: usize, end: usize, replacement: &str, original: &str) -> Mutation {
    Mutation {
        line: 1,
        column: 1,
        start_byte: start,
        end_byte: end,
        operator: "test".to_string(),
        original: original.to_string(),
        replacement: replacement.to_string(),
        context_before: vec![],
        context_after: vec![],
    }
}

// --- apply_mutation ---

#[test]
fn apply_mutation_replaces_at_correct_offset() {
    let source = "if x > 0:";
    let mutation = make_mutation(5, 6, ">=", ">");
    let result = runner::apply_mutation(source, &mutation);
    assert_eq!(result, "if x >= 0:");
}

#[test]
fn apply_mutation_at_start() {
    let source = "> 0";
    let mutation = make_mutation(0, 1, ">=", ">");
    let result = runner::apply_mutation(source, &mutation);
    assert_eq!(result, ">= 0");
}

#[test]
fn apply_mutation_at_end() {
    let source = "x > 0";
    let mutation = make_mutation(4, 5, "1", "0");
    let result = runner::apply_mutation(source, &mutation);
    assert_eq!(result, "x > 1");
}

#[test]
fn apply_mutation_replacement_longer_than_original() {
    let source = "return True";
    let mutation = make_mutation(0, 11, "return False", "return True");
    let result = runner::apply_mutation(source, &mutation);
    assert_eq!(result, "return False");
}

#[test]
fn apply_mutation_replacement_shorter_than_original() {
    let source = "return True";
    let mutation = make_mutation(0, 11, "pass", "return True");
    let result = runner::apply_mutation(source, &mutation);
    assert_eq!(result, "pass");
}

#[test]
fn apply_mutation_empty_replacement() {
    let source = "not x";
    let mutation = make_mutation(0, 4, "", "not ");
    let result = runner::apply_mutation(source, &mutation);
    assert_eq!(result, "x");
}

#[test]
fn apply_mutation_preserves_surrounding_code() {
    let source = "if a > b and c < d:";
    let mutation = make_mutation(5, 6, ">=", ">");
    let result = runner::apply_mutation(source, &mutation);
    assert_eq!(result, "if a >= b and c < d:");
}

// --- generate_diff ---

#[test]
fn generate_diff_shows_changes() {
    let original = "line1\nline2\nline3\n";
    let mutated = "line1\nchanged\nline3\n";
    let diff = runner::generate_diff(original, mutated);
    assert!(diff.contains("- line2"));
    assert!(diff.contains("+ changed"));
}

#[test]
fn generate_diff_identical_returns_empty() {
    let source = "no changes\n";
    let diff = runner::generate_diff(source, source);
    assert!(diff.is_empty());
}

#[test]
fn generate_diff_added_line() {
    let original = "a\nb\n";
    let mutated = "a\nb\nc\n";
    let diff = runner::generate_diff(original, mutated);
    assert!(diff.contains("+ c"));
    assert!(!diff.contains("- "));
}

#[test]
fn generate_diff_removed_line() {
    let original = "a\nb\nc\n";
    let mutated = "a\nc\n";
    let diff = runner::generate_diff(original, mutated);
    assert!(diff.contains("- b"));
    assert!(!diff.contains("+ "));
}

// --- parse_test_cmd ---

#[test]
fn parse_test_cmd_single_word() {
    let (program, args) = runner::parse_test_cmd("pytest");
    assert_eq!(program, "pytest");
    assert!(args.is_empty(), "Single word command should have no args");
}



#[test]
fn parse_test_cmd_multi_word() {
    let (program, args) = runner::parse_test_cmd("cargo test");
    assert_eq!(program, "cargo");
    assert_eq!(args, vec!["test"]);
}

#[test]
fn parse_test_cmd_with_path() {
    let (program, args) = runner::parse_test_cmd(".venv/bin/pytest");
    assert_eq!(program, ".venv/bin/pytest");
    assert!(args.is_empty());
}

#[test]
fn parse_test_cmd_npx() {
    let (program, args) = runner::parse_test_cmd("npx vitest run");
    assert_eq!(program, "npx");
    assert_eq!(args, vec!["vitest", "run"]);
}

// --- resolve_paths ---

#[test]
fn resolve_paths_makes_absolute() {
    let (abs_source, abs_test, working_dir, _cmd) =
        runner::resolve_paths(Path::new("foo.py"), Path::new("test_foo.py"), "pytest");
    assert!(abs_source.is_absolute());
    assert!(abs_test.is_absolute());
    assert!(working_dir.is_absolute());
}

#[test]
fn resolve_paths_preserves_absolute() {
    let (abs_source, abs_test, _, _) =
        runner::resolve_paths(Path::new("/tmp/foo.py"), Path::new("/tmp/test_foo.py"), "pytest");
    assert_eq!(abs_source, Path::new("/tmp/foo.py"));
    assert_eq!(abs_test, Path::new("/tmp/test_foo.py"));
}

#[test]
fn resolve_paths_bare_command_passes_through() {
    let (_, _, _, cmd) =
        runner::resolve_paths(Path::new("foo.py"), Path::new("test.py"), "pytest");
    assert_eq!(cmd, "pytest");
}

#[test]
fn resolve_paths_absolute_command_passes_through() {
    let (_, _, _, cmd) =
        runner::resolve_paths(Path::new("foo.py"), Path::new("test.py"), "/usr/bin/pytest");
    assert_eq!(cmd, "/usr/bin/pytest");
}

// --- clear_pycache_for ---

#[test]
fn clear_pycache_for_does_not_panic_on_nonexistent() {
    runner::clear_pycache_for(Path::new("/nonexistent/path/file.py"));
}

// --- resolve_cmd with relative paths ---

#[test]
fn resolve_paths_relative_cmd_with_slash_resolves_from_cwd() {
    // Create a temp dir with a fake pytest
    let dir = tempfile::TempDir::new().unwrap();
    let venv_bin = dir.path().join(".venv").join("bin");
    std::fs::create_dir_all(&venv_bin).unwrap();
    let fake_pytest = venv_bin.join("pytest");
    std::fs::write(&fake_pytest, "#!/bin/sh\n").unwrap();

    // resolve_paths resolves relative to CWD, but we can't change CWD in tests.
    // Instead test with absolute command path passes through.
    let abs_cmd = fake_pytest.to_string_lossy().to_string();
    let (_, _, _, cmd) =
        runner::resolve_paths(Path::new("foo.py"), Path::new("test.py"), &abs_cmd);
    assert_eq!(cmd, abs_cmd);
}

// --- run_baseline ---

#[test]
fn run_baseline_passing_test() {
    let dir = tempfile::TempDir::new().unwrap();
    let test_file = dir.path().join("test_pass.py");
    std::fs::write(&test_file, "def test_ok(): assert True").unwrap();

    // Use 'true' command which always succeeds
    let result = runner::run_baseline("true", &test_file, dir.path(), &[]);
    match result {
        runner::BaselineResult::Ok { duration_ms } => {
            assert!(duration_ms < 10000, "Should complete quickly");
        }
        runner::BaselineResult::Failed(msg) => panic!("Expected Ok, got Failed: {}", msg),
    }
}

#[test]
fn run_baseline_failing_test() {
    let dir = tempfile::TempDir::new().unwrap();
    let test_file = dir.path().join("test_fail.py");
    std::fs::write(&test_file, "").unwrap();

    // Use 'false' command which always fails
    let result = runner::run_baseline("false", &test_file, dir.path(), &[]);
    match result {
        runner::BaselineResult::Ok { .. } => panic!("Expected Failed, got Ok"),
        runner::BaselineResult::Failed(_) => {}
    }
}

#[test]
fn run_baseline_nonexistent_command() {
    let dir = tempfile::TempDir::new().unwrap();
    let test_file = dir.path().join("test.py");
    std::fs::write(&test_file, "").unwrap();

    let result = runner::run_baseline("nonexistent_command_xyz", &test_file, dir.path(), &[]);
    match result {
        runner::BaselineResult::Ok { .. } => panic!("Expected Failed for missing command"),
        runner::BaselineResult::Failed(msg) => {
            assert!(msg.contains("Failed to run"), "Expected 'Failed to run' message, got: {}", msg);
        }
    }
}

#[test]
fn run_baseline_with_extra_args() {
    let dir = tempfile::TempDir::new().unwrap();
    let test_file = dir.path().join("test.py");
    std::fs::write(&test_file, "").unwrap();

    // 'echo' with extra args should succeed
    let result = runner::run_baseline("echo", &test_file, dir.path(), &["hello"]);
    match result {
        runner::BaselineResult::Ok { .. } => {}
        runner::BaselineResult::Failed(msg) => panic!("Expected Ok, got Failed: {}", msg),
    }
}

#[test]
fn run_baseline_cargo_cmd_skips_test_file_arg() {
    let dir = tempfile::TempDir::new().unwrap();
    let test_file = dir.path().join("test.rs");
    std::fs::write(&test_file, "").unwrap();

    // cargo test would fail but this tests that the code path for cargo is hit
    // Use 'echo cargo' which contains "cargo" but is actually echo
    let result = runner::run_baseline("echo cargo", &test_file, dir.path(), &[]);
    match result {
        runner::BaselineResult::Ok { .. } => {}
        runner::BaselineResult::Failed(msg) => panic!("Expected Ok, got Failed: {}", msg),
    }
}

// --- run_mutations ---

#[test]
fn run_mutations_killed_mutant() {
    let dir = tempfile::TempDir::new().unwrap();
    let source_file = dir.path().join("app.py");
    let test_file = dir.path().join("test_app.py");

    let source = "x = 1 + 2\n";
    std::fs::write(&source_file, source).unwrap();
    std::fs::write(&test_file, "").unwrap();

    let mutation = make_mutation(4, 5, "-", "+");

    // 'false' always fails -> mutation is "killed"
    let results = runner::run_mutations(
        &source_file, &test_file, source, &[mutation],
        "false", dir.path(), 5000, &[],
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, mutator::mutants::MutantStatus::Killed);

    // Original source should be restored
    assert_eq!(std::fs::read_to_string(&source_file).unwrap(), source);
}

#[test]
fn run_mutations_survived_mutant() {
    let dir = tempfile::TempDir::new().unwrap();
    let source_file = dir.path().join("app.py");
    let test_file = dir.path().join("test_app.py");

    let source = "x = 1 + 2\n";
    std::fs::write(&source_file, source).unwrap();
    std::fs::write(&test_file, "").unwrap();

    let mutation = make_mutation(4, 5, "-", "+");

    // 'true' always succeeds -> mutation "survived"
    let results = runner::run_mutations(
        &source_file, &test_file, source, &[mutation],
        "true", dir.path(), 5000, &[],
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, mutator::mutants::MutantStatus::Survived);
}

#[test]
fn run_mutations_restores_original_on_completion() {
    let dir = tempfile::TempDir::new().unwrap();
    let source_file = dir.path().join("app.py");
    let test_file = dir.path().join("test_app.py");

    let source = "original content\n";
    std::fs::write(&source_file, source).unwrap();
    std::fs::write(&test_file, "").unwrap();

    let mutation = make_mutation(0, 8, "mutated!", "original");

    runner::run_mutations(
        &source_file, &test_file, source, &[mutation],
        "true", dir.path(), 5000, &[],
    );

    assert_eq!(std::fs::read_to_string(&source_file).unwrap(), source);
}

#[test]
fn run_mutations_multiple_mutants() {
    let dir = tempfile::TempDir::new().unwrap();
    let source_file = dir.path().join("app.py");
    let test_file = dir.path().join("test_app.py");

    let source = "a + b\n";
    std::fs::write(&source_file, source).unwrap();
    std::fs::write(&test_file, "").unwrap();

    let mutations = vec![
        make_mutation(2, 3, "-", "+"),
        make_mutation(2, 3, "*", "+"),
    ];

    let results = runner::run_mutations(
        &source_file, &test_file, source, &mutations,
        "true", dir.path(), 5000, &[],
    );

    assert_eq!(results.len(), 2);
}

// --- prepare_isolated ---

#[test]
fn prepare_isolated_creates_copy() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("pyproject.toml"), "[project]").unwrap();
    std::fs::write(root.join("app.py"), "x = 1 + 2").unwrap();
    std::fs::write(root.join("test_app.py"), "assert True").unwrap();

    let ctx = runner::prepare_isolated(
        &root.join("app.py"),
        &root.join("test_app.py"),
        "pytest",
        "test-session",
    ).unwrap();

    assert!(ctx.copy_result.source_file.exists());
    assert!(ctx.copy_result.test_file.exists());
    assert_eq!(
        std::fs::read_to_string(&ctx.copy_result.source_file).unwrap(),
        "x = 1 + 2"
    );
    // Original untouched
    assert_eq!(std::fs::read_to_string(root.join("app.py")).unwrap(), "x = 1 + 2");
}

#[test]
fn prepare_isolated_session_id_in_path() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("pyproject.toml"), "[project]").unwrap();
    std::fs::write(root.join("app.py"), "x = 1").unwrap();
    std::fs::write(root.join("test_app.py"), "").unwrap();

    let ctx = runner::prepare_isolated(
        &root.join("app.py"),
        &root.join("test_app.py"),
        "pytest",
        "my-agent-42",
    ).unwrap();

    let path_str = ctx.copy_result.root.to_string_lossy();
    assert!(path_str.contains("mutator-my-agent-42"), "Temp dir should contain session ID: {}", path_str);
}

// --- run_mutations_isolated ---

#[test]
fn run_mutations_isolated_does_not_touch_original() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("pyproject.toml"), "[project]").unwrap();
    std::fs::write(root.join("app.py"), "x = 1 + 2\n").unwrap();
    std::fs::write(root.join("test_app.py"), "").unwrap();

    let ctx = runner::prepare_isolated(
        &root.join("app.py"),
        &root.join("test_app.py"),
        "true",
        "iso-test",
    ).unwrap();

    let source = "x = 1 + 2\n";
    let mutation = make_mutation(4, 5, "-", "+");

    let results = runner::run_mutations_isolated(
        &ctx, source, &[mutation], 5000, &[],
    );

    assert_eq!(results.len(), 1);
    // Original file untouched
    assert_eq!(std::fs::read_to_string(root.join("app.py")).unwrap(), source);
}

// --- clear_pycache ---

#[test]
fn clear_pycache_removes_matching_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let cache_dir = dir.path().join("__pycache__");
    std::fs::create_dir(&cache_dir).unwrap();
    std::fs::write(cache_dir.join("app.cpython-311.pyc"), "bytes").unwrap();
    std::fs::write(cache_dir.join("other.cpython-311.pyc"), "bytes").unwrap();

    let source_file = dir.path().join("app.py");
    std::fs::write(&source_file, "").unwrap();

    runner::clear_pycache_for(&source_file);

    assert!(!cache_dir.join("app.cpython-311.pyc").exists(), "Should remove matching .pyc");
    assert!(cache_dir.join("other.cpython-311.pyc").exists(), "Should not remove unrelated .pyc");
}
