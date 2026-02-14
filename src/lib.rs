pub mod copy_tree;
pub mod mutants;
pub mod operators;
pub mod parser;
pub mod parser_js;
pub mod parser_rust;
pub mod runner;
pub mod output;
pub mod safety;
pub mod state;

pub enum Language {
    Python,
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
}

pub fn detect_language(path: &std::path::Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "py" => Some(Language::Python),
        "rs" => Some(Language::Rust),
        "js" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" | "mts" | "cts" => Some(Language::TypeScript),
        "tsx" | "jsx" => Some(Language::Tsx),
        _ => None,
    }
}

