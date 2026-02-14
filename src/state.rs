use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub score: f64,
    pub total: usize,
    pub killed: usize,
    pub survived: usize,
    pub timeout: usize,
    pub unviable: usize,
    pub duration_ms: u64,
    pub survived_mutants: Vec<SurvivedMutant>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SurvivedMutant {
    pub ref_id: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub operator: String,
    pub original: String,
    pub replacement: String,
    pub diff: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

fn state_path() -> PathBuf {
    let dir = dirs_or_cwd();
    dir.join(".mutator-state.json")
}

fn dirs_or_cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn save_last_run(result: &RunResult) {
    if let Ok(json) = serde_json::to_string(result) {
        let _ = std::fs::write(state_path(), json);
    }
}

pub fn load_last_run() -> Option<RunResult> {
    let path = state_path();
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_to_path(result: &RunResult, path: &std::path::Path) {
    if let Ok(json) = serde_json::to_string(result) {
        let _ = std::fs::write(path, json);
    }
}

pub fn load_from_path(path: &std::path::Path) -> Option<RunResult> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}
