#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use mutator::mutants::{MutantStatus, Mutation};
use mutator::runner;

const ORIGINAL: &str = "def add(a, b):\n    return a + b\n";

struct Fixture {
    dir: tempfile::TempDir,
    records: tempfile::TempDir,
    script: PathBuf,
    sibling: Child,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let records = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("app.py"), ORIGINAL).unwrap();
        fs::write(dir.path().join("test_app.py"), "").unwrap();
        fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        let script = records.path().join("tree.py");
        fs::write(
            &script,
            format!(
                r#"import os, pathlib, subprocess, sys, time
records = pathlib.Path({records:?})
role = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] in ('child', 'grandchild') else 'parent'
if role == 'parent' and pathlib.Path('app.py').read_text() == {original:?}:
    sys.exit(0)
(records / (role + '.pid')).write_text(str(os.getpid()))
if role != 'grandchild':
    subprocess.Popen([sys.executable, __file__, 'child' if role == 'parent' else 'grandchild'], env={{**os.environ, 'OBJC_DISABLE_INITIALIZE_FORK_SAFETY': 'YES'}})
time.sleep(8)
"#,
                records = records.path().to_str().unwrap(),
                original = ORIGINAL,
            ),
        )
        .unwrap();
        let sibling = Command::new("/bin/sleep")
            .arg("8")
            .env("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES")
            .spawn()
            .unwrap();
        Self {
            dir,
            records,
            script,
            sibling,
        }
    }

    fn command(&self) -> String {
        format!("/usr/bin/python3 {}", self.script.display())
    }

    fn pids(&self) -> Vec<i32> {
        ["parent", "child", "grandchild"]
            .iter()
            .filter_map(|role| {
                fs::read_to_string(self.records.path().join(format!("{role}.pid"))).ok()
            })
            .filter_map(|pid| pid.parse().ok())
            .collect()
    }

    fn assert_cleaned_up(&mut self) {
        let pids = self.pids();
        assert_eq!(
            pids.len(),
            3,
            "fixture must start all three process generations"
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while pids.iter().any(|&pid| live(pid)) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(
            pids.iter().all(|&pid| !live(pid)),
            "test command descendants remain alive: {pids:?}"
        );
        assert!(
            self.sibling.try_wait().unwrap().is_none(),
            "unrelated sibling was terminated"
        );
        assert_eq!(
            fs::read_to_string(self.dir.path().join("app.py")).unwrap(),
            ORIGINAL
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Even the unfixed implementation or a failed assertion must not leak fixtures.
        for pid in self.pids().into_iter().rev() {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
        let _ = self.sibling.kill();
        let _ = self.sibling.wait();
    }
}

fn live(pid: i32) -> bool {
    // A zombie cannot execute or consume CPU; launchd/init may reap it asynchronously.
    let output = Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .env("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES")
        .output()
        .unwrap();
    let state = String::from_utf8_lossy(&output.stdout);
    let state = state.trim();
    !state.is_empty() && !state.starts_with('Z')
}

fn timeout_tree(isolated: bool) {
    let mut fixture = Fixture::new();
    let source = fixture.dir.path().join("app.py");
    let test = fixture.dir.path().join("test_app.py");
    let start = ORIGINAL.find('+').unwrap();
    let mutation = Mutation {
        line: 2,
        column: 13,
        start_byte: start,
        end_byte: start + 1,
        operator: "arithmetic".into(),
        original: "+".into(),
        replacement: "-".into(),
        context_before: vec![],
        context_after: vec![],
    };
    let started = Instant::now();
    let results = if isolated {
        let ctx =
            runner::prepare_isolated(&source, &test, &fixture.command(), "cleanup-test").unwrap();
        let result = runner::run_mutations_isolated(&ctx, ORIGINAL, &[mutation], 1000, &[]);
        assert_eq!(
            fs::read_to_string(&ctx.copy_result.source_file).unwrap(),
            ORIGINAL
        );
        result
    } else {
        runner::run_mutations(
            &source,
            &test,
            ORIGINAL,
            &[mutation],
            &fixture.command(),
            fixture.dir.path(),
            1000,
            &[],
        )
    };
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "timeout waited for fixture failsafe"
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, MutantStatus::Timeout);
    fixture.assert_cleaned_up();
}

#[test]
fn timeout_cleans_process_tree_in_place() {
    timeout_tree(false);
}

#[test]
fn timeout_cleans_process_tree_isolated() {
    timeout_tree(true);
}

struct CliChild(Child);
impl Drop for CliChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn cancel_tree(isolated: bool, signal: i32) {
    let mut fixture = Fixture::new();
    let mut command = Command::new(env!("CARGO_BIN_EXE_mutator"));
    command
        .current_dir(fixture.dir.path())
        .args([
            "run",
            "app.py",
            "--test",
            "test_app.py",
            "--function",
            "add",
            "--test-cmd",
        ])
        .arg(fixture.command())
        .args(["--timeout-mult", "1000", "--json"])
        .env("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !isolated {
        command.arg("--in-place");
    }
    let mut child = CliChild(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(3);
    while fixture.pids().len() < 3 && Instant::now() < deadline {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "mutator exited before mutation tree started"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fixture.pids().len(), 3, "mutation tree failed to start");
    assert_eq!(unsafe { libc::kill(child.0.id() as i32, signal) }, 0);
    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "mutator did not respond to cancellation"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(status.code(), Some(128 + signal));
    fixture.assert_cleaned_up();
    assert!(!fixture.dir.path().join(".app.py.mutator.bak").exists());
}

#[test]
fn sigint_cleans_process_tree_in_place() {
    cancel_tree(false, libc::SIGINT);
}
#[test]
fn sigint_cleans_process_tree_isolated() {
    cancel_tree(true, libc::SIGINT);
}
#[test]
fn sigterm_cleans_process_tree_in_place() {
    cancel_tree(false, libc::SIGTERM);
}
#[test]
fn sigterm_cleans_process_tree_isolated() {
    cancel_tree(true, libc::SIGTERM);
}

#[test]
fn sighup_cleans_process_tree_in_place() {
    cancel_tree(false, libc::SIGHUP);
}

#[test]
fn sighup_cleans_process_tree_isolated() {
    cancel_tree(true, libc::SIGHUP);
}
