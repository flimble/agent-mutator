use std::fs;
use std::path::{Path, PathBuf};

const SKIP_NAMES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "target",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".mutator-state.json",
];

const SKIP_SUFFIXES: &[&str] = &[
    ".mutator.bak",
    ".pyc",
    ".pyo",
];

pub struct CopyResult {
    pub root: PathBuf,
    pub source_file: PathBuf,
    pub test_file: PathBuf,
}

fn should_skip(name: &str) -> bool {
    SKIP_NAMES.iter().any(|s| *s == name)
        || SKIP_SUFFIXES.iter().any(|s| name.ends_with(s))
}

fn copy_dir_filtered(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if should_skip(&name_str) {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_filtered(&src_path, &dst_path)?;
        } else if ft.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
        // Skip symlinks and other special files
    }
    Ok(())
}

/// Find the project root by walking up from source_file looking for markers.
pub fn find_project_root(source_file: &Path) -> PathBuf {
    let markers = &[
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "package.json",
        "Cargo.toml",
        "go.mod",
        ".git",
    ];
    let mut dir = source_file.parent().unwrap_or(source_file);
    loop {
        for marker in markers {
            if dir.join(marker).exists() {
                return dir.to_path_buf();
            }
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent,
            _ => break,
        }
    }
    source_file
        .parent()
        .unwrap_or(source_file)
        .to_path_buf()
}

/// Copy the project tree to a temp directory, returning paths mapped into the copy.
pub fn copy_tree(
    project_root: &Path,
    source_file: &Path,
    test_file: &Path,
    dest_root: &Path,
) -> std::io::Result<CopyResult> {
    copy_dir_filtered(project_root, dest_root)?;

    let rel_source = source_file
        .strip_prefix(project_root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let rel_test = test_file
        .strip_prefix(project_root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    Ok(CopyResult {
        root: dest_root.to_path_buf(),
        source_file: dest_root.join(rel_source),
        test_file: dest_root.join(rel_test),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn copy_tree_copies_files_and_skips_git() {
        let src_dir = TempDir::new().unwrap();
        let src = src_dir.path();
        fs::write(src.join("app.py"), "x = 1").unwrap();
        fs::write(src.join("test_app.py"), "assert True").unwrap();
        fs::create_dir(src.join(".git")).unwrap();
        fs::write(src.join(".git").join("HEAD"), "ref").unwrap();
        fs::create_dir(src.join("__pycache__")).unwrap();
        fs::write(src.join("__pycache__").join("app.cpython-311.pyc"), "bytes").unwrap();

        let dst_dir = TempDir::new().unwrap();
        let result = copy_tree(
            src,
            &src.join("app.py"),
            &src.join("test_app.py"),
            dst_dir.path(),
        )
        .unwrap();

        assert!(result.source_file.exists());
        assert!(result.test_file.exists());
        assert!(!dst_dir.path().join(".git").exists());
        assert!(!dst_dir.path().join("__pycache__").exists());
    }

    #[test]
    fn copy_tree_preserves_nested_structure() {
        let src_dir = TempDir::new().unwrap();
        let src = src_dir.path();
        fs::create_dir_all(src.join("src").join("utils")).unwrap();
        fs::write(src.join("src").join("utils").join("math.py"), "def add(a,b): return a+b").unwrap();
        fs::write(src.join("test_math.py"), "pass").unwrap();
        fs::write(src.join("pyproject.toml"), "[project]").unwrap();

        let dst_dir = TempDir::new().unwrap();
        let result = copy_tree(
            src,
            &src.join("src").join("utils").join("math.py"),
            &src.join("test_math.py"),
            dst_dir.path(),
        )
        .unwrap();

        assert!(result.source_file.exists());
        assert_eq!(
            fs::read_to_string(&result.source_file).unwrap(),
            "def add(a,b): return a+b"
        );
    }

    #[test]
    fn find_project_root_finds_pyproject() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("pyproject.toml"), "").unwrap();
        fs::write(root.join("src").join("app.py"), "").unwrap();

        let found = find_project_root(&root.join("src").join("app.py"));
        assert_eq!(found, root);
    }

    #[test]
    fn find_project_root_finds_package_json() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("package.json"), "{}").unwrap();
        fs::write(root.join("src").join("index.ts"), "").unwrap();

        let found = find_project_root(&root.join("src").join("index.ts"));
        assert_eq!(found, root);
    }

    #[test]
    fn find_project_root_fallback_to_parent() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        // No marker files at all - should fall back to parent dir
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src").join("app.py"), "").unwrap();

        let found = find_project_root(&root.join("src").join("app.py"));
        // Should return the parent directory of the file
        assert_eq!(found, root.join("src"));
    }

    #[test]
    fn copy_tree_skips_pyo_files() {
        let src_dir = TempDir::new().unwrap();
        let src = src_dir.path();
        fs::write(src.join("app.py"), "x = 1").unwrap();
        fs::write(src.join("test.py"), "pass").unwrap();
        fs::write(src.join("compiled.pyo"), "bytes").unwrap();

        let dst_dir = TempDir::new().unwrap();
        copy_tree(src, &src.join("app.py"), &src.join("test.py"), dst_dir.path()).unwrap();

        assert!(!dst_dir.path().join("compiled.pyo").exists());
    }

    #[test]
    fn copy_tree_skips_all_filtered_dirs() {
        let src_dir = TempDir::new().unwrap();
        let src = src_dir.path();
        fs::write(src.join("app.py"), "x = 1").unwrap();
        fs::write(src.join("test.py"), "pass").unwrap();

        for dir_name in &[".hg", ".svn", "node_modules", ".venv", "venv", ".tox",
                          ".mypy_cache", ".pytest_cache", ".ruff_cache", "dist",
                          "build", ".next", ".nuxt"] {
            fs::create_dir(src.join(dir_name)).unwrap();
            fs::write(src.join(dir_name).join("file"), "data").unwrap();
        }

        let dst_dir = TempDir::new().unwrap();
        copy_tree(src, &src.join("app.py"), &src.join("test.py"), dst_dir.path()).unwrap();

        for dir_name in &[".hg", ".svn", "node_modules", ".venv", "venv", ".tox",
                          ".mypy_cache", ".pytest_cache", ".ruff_cache", "dist",
                          "build", ".next", ".nuxt"] {
            assert!(!dst_dir.path().join(dir_name).exists(), "{} should be skipped", dir_name);
        }
    }

    #[test]
    fn find_project_root_finds_cargo_toml() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"), "[package]").unwrap();
        fs::write(root.join("src").join("lib.rs"), "").unwrap();

        let found = find_project_root(&root.join("src").join("lib.rs"));
        assert_eq!(found, root);
    }

    #[test]
    fn find_project_root_finds_git_dir() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join("src").join("main.py"), "").unwrap();

        let found = find_project_root(&root.join("src").join("main.py"));
        assert_eq!(found, root);
    }

    #[test]
    fn should_skip_filters_correctly() {
        assert!(should_skip(".git"));
        assert!(should_skip("node_modules"));
        assert!(should_skip("__pycache__"));
        assert!(should_skip("target"));
        assert!(should_skip("foo.mutator.bak"));
        assert!(should_skip("app.pyc"));
        assert!(!should_skip("app.py"));
        assert!(!should_skip("src"));
        assert!(!should_skip("Cargo.toml"));
    }
}
