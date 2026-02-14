use mutator::state::{self, RunResult, SurvivedMutant};
use tempfile::TempDir;

#[test]
fn run_result_serializes_to_json() {
    let result = RunResult {
        score: 0.85,
        total: 20,
        killed: 17,
        survived: 3,
        timeout: 0,
        unviable: 0,
        duration_ms: 5000,
        survived_mutants: vec![
            SurvivedMutant {
                ref_id: "m1".into(),
                file: "test.py".into(),
                line: 10,
                column: 5,
                operator: "boundary".into(),
                original: ">".into(),
                replacement: ">=".into(),
                diff: "- x > 0\n+ x >= 0\n".into(),
                context_before: vec!["line before".into()],
                context_after: vec!["line after".into()],
            },
        ],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"score\":0.85"));
    assert!(json.contains("\"total\":20"));
    assert!(json.contains("\"ref_id\":\"m1\""));
}

#[test]
fn run_result_roundtrips_through_json() {
    let result = RunResult {
        score: 1.0,
        total: 5,
        killed: 5,
        survived: 0,
        timeout: 0,
        unviable: 0,
        duration_ms: 1234,
        survived_mutants: vec![],
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: RunResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.score, 1.0);
    assert_eq!(deserialized.total, 5);
    assert_eq!(deserialized.killed, 5);
    assert_eq!(deserialized.survived, 0);
    assert_eq!(deserialized.duration_ms, 1234);
    assert!(deserialized.survived_mutants.is_empty());
}

#[test]
fn survived_mutant_serializes_all_fields() {
    let mutant = SurvivedMutant {
        ref_id: "m3".into(),
        file: "app.py".into(),
        line: 42,
        column: 8,
        operator: "negate_eq".into(),
        original: "==".into(),
        replacement: "!=".into(),
        diff: "- x == 0\n+ x != 0\n".into(),
        context_before: vec!["before1".into(), "before2".into()],
        context_after: vec!["after1".into()],
    };

    let json = serde_json::to_string(&mutant).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["ref_id"], "m3");
    assert_eq!(parsed["file"], "app.py");
    assert_eq!(parsed["line"], 42);
    assert_eq!(parsed["column"], 8);
    assert_eq!(parsed["operator"], "negate_eq");
    assert_eq!(parsed["original"], "==");
    assert_eq!(parsed["replacement"], "!=");
    assert_eq!(parsed["context_before"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["context_after"].as_array().unwrap().len(), 1);
}

#[test]
fn run_result_with_survivors_roundtrips() {
    let result = RunResult {
        score: 0.5,
        total: 4,
        killed: 2,
        survived: 2,
        timeout: 0,
        unviable: 0,
        duration_ms: 10000,
        survived_mutants: vec![
            SurvivedMutant {
                ref_id: "m1".into(),
                file: "src/lib.rs".into(),
                line: 10,
                column: 5,
                operator: "boundary".into(),
                original: ">".into(),
                replacement: ">=".into(),
                diff: "- x > 0\n+ x >= 0\n".into(),
                context_before: vec![],
                context_after: vec![],
            },
            SurvivedMutant {
                ref_id: "m2".into(),
                file: "src/lib.rs".into(),
                line: 20,
                column: 3,
                operator: "bool_flip".into(),
                original: "true".into(),
                replacement: "false".into(),
                diff: "- true\n+ false\n".into(),
                context_before: vec!["fn check()".into()],
                context_after: vec!["return x".into()],
            },
        ],
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: RunResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.survived_mutants.len(), 2);
    assert_eq!(deserialized.survived_mutants[0].ref_id, "m1");
    assert_eq!(deserialized.survived_mutants[1].ref_id, "m2");
    assert_eq!(deserialized.survived_mutants[1].operator, "bool_flip");
}

// --- File I/O tests ---

#[test]
fn save_and_load_roundtrip_via_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".mutator-state.json");

    let result = RunResult {
        score: 0.75,
        total: 8,
        killed: 6,
        survived: 2,
        timeout: 0,
        unviable: 0,
        duration_ms: 3000,
        survived_mutants: vec![
            SurvivedMutant {
                ref_id: "m1".into(),
                file: "test.py".into(),
                line: 5,
                column: 3,
                operator: "boundary".into(),
                original: ">".into(),
                replacement: ">=".into(),
                diff: "- x > 0\n+ x >= 0\n".into(),
                context_before: vec![],
                context_after: vec![],
            },
        ],
    };

    state::save_to_path(&result, &path);
    assert!(path.exists(), "State file should be created");

    let loaded = state::load_from_path(&path).expect("Should load saved state");
    assert_eq!(loaded.score, 0.75);
    assert_eq!(loaded.total, 8);
    assert_eq!(loaded.killed, 6);
    assert_eq!(loaded.survived, 2);
    assert_eq!(loaded.survived_mutants.len(), 1);
    assert_eq!(loaded.survived_mutants[0].ref_id, "m1");
}

#[test]
fn load_from_nonexistent_path_returns_none() {
    let result = state::load_from_path(std::path::Path::new("/nonexistent/path/state.json"));
    assert!(result.is_none());
}

#[test]
fn load_from_invalid_json_returns_none() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, "not valid json").unwrap();

    let result = state::load_from_path(&path);
    assert!(result.is_none());
}

#[test]
fn save_empty_result_and_load() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".mutator-state.json");

    let result = RunResult {
        score: 1.0,
        total: 0,
        killed: 0,
        survived: 0,
        timeout: 0,
        unviable: 0,
        duration_ms: 0,
        survived_mutants: vec![],
    };

    state::save_to_path(&result, &path);
    let loaded = state::load_from_path(&path).unwrap();
    assert_eq!(loaded.score, 1.0);
    assert_eq!(loaded.total, 0);
    assert!(loaded.survived_mutants.is_empty());
}

// --- save_last_run / load_last_run (CWD-based) ---

#[test]
fn save_last_run_writes_file_to_cwd() {
    let dir = TempDir::new().unwrap();
    let result = RunResult {
        score: 0.9,
        total: 10,
        killed: 9,
        survived: 1,
        timeout: 0,
        unviable: 0,
        duration_ms: 2000,
        survived_mutants: vec![],
    };

    // Change CWD to temp dir so save_last_run writes there
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    state::save_last_run(&result);

    let state_file = dir.path().join(".mutator-state.json");
    assert!(state_file.exists(), "save_last_run should create .mutator-state.json in CWD");

    let loaded = state::load_last_run().unwrap();
    assert_eq!(loaded.score, 0.9);
    assert_eq!(loaded.total, 10);
    assert_eq!(loaded.killed, 9);

    std::env::set_current_dir(original_dir).unwrap();
}
