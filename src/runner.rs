use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::copy_tree::{self, CopyResult};
use crate::mutants::{Mutation, MutantResult, MutantStatus};

pub enum BaselineResult {
    Ok { duration_ms: u64 },
    Failed(String),
}

pub struct IsolatedContext {
    pub copy_result: CopyResult,
    pub resolved_cmd: String,
    pub _temp_dir: tempfile::TempDir,
}

/// Resolve all paths to absolute. This is critical for flat project layouts
/// where source, tests, and venv all live in the same directory. We never
/// copy files elsewhere (unlike mutmut's mutants/ dir approach), so imports
/// always work.
pub fn resolve_paths(
    source_file: &Path,
    test_file: &Path,
    test_cmd: &str,
) -> (PathBuf, PathBuf, PathBuf, String) {
    let cwd = std::env::current_dir().expect("Failed to get current directory");

    let abs_source = if source_file.is_absolute() {
        source_file.to_path_buf()
    } else {
        cwd.join(source_file)
    };

    let abs_test = if test_file.is_absolute() {
        test_file.to_path_buf()
    } else {
        cwd.join(test_file)
    };

    let working_dir = abs_source
        .parent()
        .unwrap_or(&cwd)
        .to_path_buf();

    let resolved_cmd = resolve_cmd(test_cmd, &working_dir, &cwd);

    (abs_source, abs_test, working_dir, resolved_cmd)
}

pub fn parse_test_cmd(cmd: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.len() > 1 {
        (parts[0].to_string(), parts[1..].iter().map(|s| s.to_string()).collect())
    } else {
        (cmd.to_string(), vec![])
    }
}

fn resolve_cmd(cmd: &str, working_dir: &Path, cwd: &Path) -> String {
    let p = Path::new(cmd);
    if p.is_absolute() {
        return cmd.to_string();
    }
    // Try resolving relative to CWD first (most common: .venv/bin/pytest from project root)
    if cmd.contains('/') {
        let from_cwd = cwd.join(p);
        if from_cwd.exists() {
            return from_cwd.to_string_lossy().to_string();
        }
        let from_wd = working_dir.join(p);
        if from_wd.exists() {
            return from_wd.to_string_lossy().to_string();
        }
    }
    // Bare command (e.g. "pytest") — let PATH resolve it
    cmd.to_string()
}

pub fn run_baseline(test_cmd: &str, test_file: &Path, working_dir: &Path, extra_args: &[&str]) -> BaselineResult {
    let start = Instant::now();
    let (program, first_args) = parse_test_cmd(test_cmd);
    let mut cmd = Command::new(&program);
    for arg in &first_args {
        cmd.arg(arg);
    }
    // For non-cargo commands, pass test file as arg
    if !test_cmd.contains("cargo") {
        cmd.arg(test_file);
    }
    for arg in extra_args {
        cmd.arg(arg);
    }
    let output = cmd
        .current_dir(working_dir)
        .env("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES")
        .output();

    match output {
        Ok(o) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            if o.status.success() {
                BaselineResult::Ok { duration_ms }
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                BaselineResult::Failed(format!("{}\n{}", stdout, stderr))
            }
        }
        Err(e) => BaselineResult::Failed(format!("Failed to run {}: {}", test_cmd, e)),
    }
}

pub fn run_mutations(
    source_file: &Path,
    test_file: &Path,
    original_source: &str,
    mutations: &[Mutation],
    test_cmd: &str,
    working_dir: &Path,
    timeout_ms: u64,
    extra_args: &[&str],
) -> Vec<MutantResult> {
    let mut results = Vec::with_capacity(mutations.len());

    for mutation in mutations {
        let mutated = apply_mutation(original_source, mutation);
        let diff = generate_diff(original_source, &mutated);

        if std::fs::write(source_file, &mutated).is_err() {
            results.push(MutantResult {
                mutation: mutation.clone(),
                status: MutantStatus::Unviable,
                duration_ms: 0,
                diff,
            });
            continue;
        }

        let start = Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        clear_pycache(source_file);

        let (program, first_args) = parse_test_cmd(test_cmd);
        let mut cmd = Command::new(&program);
        for arg in &first_args {
            cmd.arg(arg);
        }
        if !test_cmd.contains("cargo") {
            cmd.arg(test_file);
        }
        for arg in extra_args {
            cmd.arg(arg);
        }
        let child = cmd
            .current_dir(working_dir)
            .env("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let status = match child {
            Ok(mut child) => {
                loop {
                    match child.try_wait() {
                        Ok(Some(exit_status)) => {
                            let stderr = child
                                .stderr
                                .take()
                                .and_then(|mut s| {
                                    let mut buf = String::new();
                                    std::io::Read::read_to_string(&mut s, &mut buf).ok()?;
                                    Some(buf)
                                })
                                .unwrap_or_default();

                            if exit_status.success() {
                                break MutantStatus::Survived;
                            } else if stderr.contains("SyntaxError")
                                || stderr.contains("IndentationError")
                                || stderr.contains("ImportError")
                                || stderr.contains("ModuleNotFoundError")
                            {
                                break MutantStatus::Unviable;
                            } else {
                                break MutantStatus::Killed;
                            }
                        }
                        Ok(None) => {
                            if start.elapsed() > timeout {
                                let _ = child.kill();
                                let _ = child.wait();
                                break MutantStatus::Timeout;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        Err(_) => break MutantStatus::Unviable,
                    }
                }
            }
            Err(_) => MutantStatus::Unviable,
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        results.push(MutantResult {
            mutation: mutation.clone(),
            status,
            duration_ms,
            diff,
        });
    }

    // ALWAYS restore original source, even on panic
    let _ = std::fs::write(source_file, original_source);
    // Restore pycache validity
    clear_pycache(source_file);

    results
}

/// Public wrapper for signal handler access.
pub fn clear_pycache_for(source_file: &Path) {
    clear_pycache(source_file);
}

/// Remove the __pycache__ .pyc file for a given source file.
/// This forces Python to re-read the .py file on next import.
fn clear_pycache(source_file: &Path) {
    if let Some(parent) = source_file.parent() {
        if let Some(stem) = source_file.file_stem() {
            let cache_dir = parent.join("__pycache__");
            if cache_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                    let stem_str = stem.to_string_lossy();
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with(&*stem_str) && name_str.ends_with(".pyc") {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
}

/// Set up an isolated copy of the project tree for safe mutation testing.
/// The original source is never modified.
pub fn prepare_isolated(
    abs_source: &Path,
    abs_test: &Path,
    test_cmd: &str,
    session_id: &str,
) -> Result<IsolatedContext, String> {
    let project_root = copy_tree::find_project_root(abs_source);
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let temp_dir = tempfile::Builder::new()
        .prefix(&format!("mutator-{}-", session_id))
        .tempdir()
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let copy_result = copy_tree::copy_tree(
        &project_root,
        abs_source,
        abs_test,
        temp_dir.path(),
    )
    .map_err(|e| format!("Failed to copy project tree: {}", e))?;

    // Resolve test command: if it's a relative path that exists in the original CWD,
    // use the absolute path so it works from the copied tree.
    let resolved_cmd = resolve_cmd(test_cmd, &copy_result.root, &cwd);

    Ok(IsolatedContext {
        copy_result,
        resolved_cmd,
        _temp_dir: temp_dir,
    })
}

/// Run mutations in an isolated copy. Original source is never touched.
pub fn run_mutations_isolated(
    ctx: &IsolatedContext,
    original_source: &str,
    mutations: &[Mutation],
    timeout_ms: u64,
    extra_args: &[&str],
) -> Vec<MutantResult> {
    let source_file = &ctx.copy_result.source_file;
    let test_file = &ctx.copy_result.test_file;
    let working_dir = &ctx.copy_result.root;
    let test_cmd = &ctx.resolved_cmd;

    let mut results = Vec::with_capacity(mutations.len());

    for mutation in mutations {
        let mutated = apply_mutation(original_source, mutation);
        let diff = generate_diff(original_source, &mutated);

        if std::fs::write(source_file, &mutated).is_err() {
            results.push(MutantResult {
                mutation: mutation.clone(),
                status: MutantStatus::Unviable,
                duration_ms: 0,
                diff,
            });
            continue;
        }

        let start = Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        clear_pycache(source_file);

        let (program, first_args) = parse_test_cmd(test_cmd);
        let mut cmd = Command::new(&program);
        for arg in &first_args {
            cmd.arg(arg);
        }
        if !test_cmd.contains("cargo") {
            cmd.arg(test_file);
        }
        for arg in extra_args {
            cmd.arg(arg);
        }
        let child = cmd
            .current_dir(working_dir)
            .env("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let status = match child {
            Ok(mut child) => {
                loop {
                    match child.try_wait() {
                        Ok(Some(exit_status)) => {
                            let stderr = child
                                .stderr
                                .take()
                                .and_then(|mut s| {
                                    let mut buf = String::new();
                                    std::io::Read::read_to_string(&mut s, &mut buf).ok()?;
                                    Some(buf)
                                })
                                .unwrap_or_default();

                            if exit_status.success() {
                                break MutantStatus::Survived;
                            } else if stderr.contains("SyntaxError")
                                || stderr.contains("IndentationError")
                                || stderr.contains("ImportError")
                                || stderr.contains("ModuleNotFoundError")
                            {
                                break MutantStatus::Unviable;
                            } else {
                                break MutantStatus::Killed;
                            }
                        }
                        Ok(None) => {
                            if start.elapsed() > timeout {
                                let _ = child.kill();
                                let _ = child.wait();
                                break MutantStatus::Timeout;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        Err(_) => break MutantStatus::Unviable,
                    }
                }
            }
            Err(_) => MutantStatus::Unviable,
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        results.push(MutantResult {
            mutation: mutation.clone(),
            status,
            duration_ms,
            diff,
        });

        // Restore original in the copy for the next mutation
        let _ = std::fs::write(source_file, original_source);
        clear_pycache(source_file);
    }

    results
}

pub fn apply_mutation(source: &str, mutation: &Mutation) -> String {
    let mut result = String::with_capacity(source.len());
    result.push_str(&source[..mutation.start_byte]);
    result.push_str(&mutation.replacement);
    result.push_str(&source[mutation.end_byte..]);
    result
}

pub fn generate_diff(original: &str, mutated: &str) -> String {
    use similar::TextDiff;
    let diff = TextDiff::from_lines(original, mutated);
    let mut output = String::new();
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Delete => {
                output.push_str(&format!("- {}", change));
            }
            similar::ChangeTag::Insert => {
                output.push_str(&format!("+ {}", change));
            }
            _ => {}
        }
    }
    output
}
