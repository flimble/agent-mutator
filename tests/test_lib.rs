use std::path::Path;

#[test]
fn detect_python() {
    assert!(matches!(mutator::detect_language(Path::new("foo.py")), Some(mutator::Language::Python)));
}

#[test]
fn detect_rust() {
    assert!(matches!(mutator::detect_language(Path::new("foo.rs")), Some(mutator::Language::Rust)));
}

#[test]
fn detect_javascript() {
    assert!(matches!(mutator::detect_language(Path::new("foo.js")), Some(mutator::Language::JavaScript)));
    assert!(matches!(mutator::detect_language(Path::new("foo.mjs")), Some(mutator::Language::JavaScript)));
    assert!(matches!(mutator::detect_language(Path::new("foo.cjs")), Some(mutator::Language::JavaScript)));
}

#[test]
fn detect_typescript() {
    assert!(matches!(mutator::detect_language(Path::new("foo.ts")), Some(mutator::Language::TypeScript)));
    assert!(matches!(mutator::detect_language(Path::new("foo.mts")), Some(mutator::Language::TypeScript)));
    assert!(matches!(mutator::detect_language(Path::new("foo.cts")), Some(mutator::Language::TypeScript)));
}

#[test]
fn detect_tsx_jsx() {
    assert!(matches!(mutator::detect_language(Path::new("foo.tsx")), Some(mutator::Language::Tsx)));
    assert!(matches!(mutator::detect_language(Path::new("foo.jsx")), Some(mutator::Language::Tsx)));
}

#[test]
fn detect_unknown_returns_none() {
    assert!(mutator::detect_language(Path::new("foo.go")).is_none());
    assert!(mutator::detect_language(Path::new("foo.java")).is_none());
    assert!(mutator::detect_language(Path::new("foo")).is_none());
}
