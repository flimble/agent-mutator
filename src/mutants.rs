use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub line: usize,
    pub column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub operator: String,
    pub original: String,
    pub replacement: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MutantStatus {
    Killed,
    Survived,
    Timeout,
    Unviable,
}

#[derive(Debug, Clone)]
pub struct MutantResult {
    pub mutation: Mutation,
    pub status: MutantStatus,
    pub duration_ms: u64,
    pub diff: String,
}
