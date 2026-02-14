use mutator::mutants;
use mutator::parser;
use mutator::parser_js;
use mutator::parser_rust;
use mutator::runner;
use mutator::output;
use mutator::safety;
use mutator::state;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mutator", version, about = "Mutation testing for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run mutation testing on a source file
    Run {
        /// Source file to mutate
        file: PathBuf,
        /// Test file to run against mutations
        #[arg(short, long)]
        test: PathBuf,
        /// Function name to scope mutations to (recommended)
        #[arg(short, long)]
        function: Option<String>,
        /// Output JSON instead of human-readable text
        #[arg(long)]
        json: bool,
        /// Exit code only, no output
        #[arg(short, long)]
        quiet: bool,
        /// Only mutate lines changed in git
        #[arg(long)]
        in_diff: bool,
        /// Test command override (default: pytest)
        #[arg(long, default_value = "pytest")]
        test_cmd: String,
        /// Timeout multiplier for test runs (default: 3x baseline)
        #[arg(long, default_value = "3")]
        timeout_mult: f64,
        /// Session ID for isolation (default: auto-generated). Agents should pass their own.
        #[arg(long)]
        session: Option<String>,
        /// Mutate source in-place instead of copying to temp dir (unsafe for concurrent use)
        #[arg(long)]
        in_place: bool,
    },
    /// Show details for a survived mutant by ref
    Show {
        /// Mutant ref (e.g. @m1 or m1)
        #[arg(name = "ref")]
        mutant_ref: String,
        /// Output JSON
        #[arg(long)]
        json: bool,
    },
    /// Summary of last run
    Status {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Run {
            file,
            test,
            function,
            json,
            quiet,
            in_diff,
            test_cmd,
            timeout_mult,
            session,
            in_place,
        } => cmd_run(file, test, function, json, quiet, in_diff, test_cmd, timeout_mult, session, in_place),
        Commands::Show { mutant_ref, json } => cmd_show(mutant_ref, json),
        Commands::Status { json } => cmd_status(json),
    };

    process::exit(exit_code);
}

fn generate_session_id() -> String {
    format!("{:08x}", fastrand::u32(..))
}

fn cmd_run(
    file: PathBuf,
    test: PathBuf,
    function: Option<String>,
    json_mode: bool,
    quiet: bool,
    _in_diff: bool,
    test_cmd: String,
    timeout_mult: f64,
    session: Option<String>,
    in_place: bool,
) -> i32 {
    let (abs_file, abs_test, _working_dir, resolved_cmd) =
        runner::resolve_paths(&file, &test, &test_cmd);

    // Legacy: recover from a previously interrupted in-place run
    if let Some(bak_path) = safety::check_interrupted_run(&abs_file) {
        if safety::restore_from_backup(&abs_file, &bak_path).is_ok() {
            output::print_error(
                "Recovered source file from a previously interrupted run. Re-run to continue."
            );
            return 3;
        }
    }

    if !abs_file.exists() {
        output::print_error(&format!(
            "Source file not found: {}. Check the path and try again.",
            abs_file.display()
        ));
        return 2;
    }
    if !abs_test.exists() {
        output::print_error(&format!(
            "Test file not found: {}. Pass --test <path> with a valid test file.",
            abs_test.display()
        ));
        return 2;
    }

    let source = match std::fs::read_to_string(&abs_file) {
        Ok(s) => s,
        Err(e) => {
            output::print_error(&format!("Failed to read {}: {}", abs_file.display(), e));
            return 3;
        }
    };

    let lang = match mutator::detect_language(&abs_file) {
        Some(l) => l,
        None => {
            output::print_error(&format!(
                "Unsupported file type: {}. Supported: .py, .rs, .js, .ts, .tsx, .jsx",
                abs_file.display()
            ));
            return 2;
        }
    };

    if let Some(ref fn_name) = function {
        let available = match lang {
            mutator::Language::Python => parser::list_functions(&source),
            mutator::Language::Rust => parser_rust::list_functions(&source),
            mutator::Language::JavaScript => parser_js::list_functions(&source, parser_js::JsDialect::JavaScript),
            mutator::Language::TypeScript => parser_js::list_functions(&source, parser_js::JsDialect::TypeScript),
            mutator::Language::Tsx => parser_js::list_functions(&source, parser_js::JsDialect::Tsx),
        };
        if !available.iter().any(|n| n == fn_name) {
            output::print_error(&format!(
                "Function '{}' not found. Available: {}",
                fn_name,
                available.join(", ")
            ));
            return 2;
        }
    }

    let mutations = match lang {
        mutator::Language::Python => parser::discover_mutations(&source, function.as_deref()),
        mutator::Language::Rust => parser_rust::discover_mutations(&source, function.as_deref()),
        mutator::Language::JavaScript => parser_js::discover_mutations(&source, function.as_deref(), parser_js::JsDialect::JavaScript),
        mutator::Language::TypeScript => parser_js::discover_mutations(&source, function.as_deref(), parser_js::JsDialect::TypeScript),
        mutator::Language::Tsx => parser_js::discover_mutations(&source, function.as_deref(), parser_js::JsDialect::Tsx),
    };
    if mutations.is_empty() {
        if !quiet {
            if json_mode {
                let result = state::RunResult {
                    score: 1.0,
                    total: 0,
                    killed: 0,
                    survived: 0,
                    timeout: 0,
                    unviable: 0,
                    duration_ms: 0,
                    survived_mutants: vec![],
                };
                println!("{}", serde_json::to_string(&result).unwrap());
            } else {
                output::print_success("No mutable code found.");
            }
        }
        return 0;
    }

    let (baseline_args, mutation_args): (Vec<&str>, Vec<&str>) = match lang {
        mutator::Language::Python => (
            vec!["-x", "-q", "--tb=short", "--no-header"],
            vec!["-x", "-q", "--tb=no", "--no-header", "-p", "no:cacheprovider"],
        ),
        mutator::Language::Rust => (
            vec!["--", "--test-threads=1"],
            vec!["--", "--test-threads=1"],
        ),
        mutator::Language::JavaScript | mutator::Language::TypeScript | mutator::Language::Tsx => (
            vec!["--bail"],
            vec!["--bail"],
        ),
    };

    if in_place {
        return run_in_place(
            &abs_file, &abs_test, &source, &mutations, &resolved_cmd,
            &_working_dir, &baseline_args, &mutation_args,
            timeout_mult, json_mode, quiet, &file,
        );
    }

    // Default: isolated tree-copy mode
    let session_id = session.unwrap_or_else(generate_session_id);

    let ctx = match runner::prepare_isolated(&abs_file, &abs_test, &test_cmd, &session_id) {
        Ok(c) => c,
        Err(e) => {
            output::print_error(&format!("Failed to set up isolated environment: {}", e));
            return 3;
        }
    };

    let baseline = runner::run_baseline(
        &ctx.resolved_cmd,
        &ctx.copy_result.test_file,
        &ctx.copy_result.root,
        &baseline_args,
    );
    match baseline {
        runner::BaselineResult::Failed(stderr) => {
            output::print_error(&format!(
                "Tests fail before mutation. Fix failing tests first.\n{}",
                stderr
            ));
            3
        }
        runner::BaselineResult::Ok { duration_ms } => {
            let timeout_ms = (duration_ms as f64 * timeout_mult) as u64 + 2000;

            let results = runner::run_mutations_isolated(
                &ctx,
                &source,
                &mutations,
                timeout_ms,
                &mutation_args,
            );

            finalize_results(&results, &mutations, &file, json_mode, quiet)
        }
    }
}

/// Legacy in-place mutation mode (--in-place flag)
fn run_in_place(
    abs_file: &std::path::Path,
    abs_test: &std::path::Path,
    source: &str,
    mutations: &[mutator::mutants::Mutation],
    resolved_cmd: &str,
    working_dir: &std::path::Path,
    baseline_args: &[&str],
    mutation_args: &[&str],
    timeout_mult: f64,
    json_mode: bool,
    quiet: bool,
    display_file: &std::path::Path,
) -> i32 {
    let baseline = runner::run_baseline(resolved_cmd, abs_test, working_dir, baseline_args);
    match baseline {
        runner::BaselineResult::Failed(stderr) => {
            output::print_error(&format!(
                "Tests fail before mutation. Fix failing tests first.\n{}",
                stderr
            ));
            3
        }
        runner::BaselineResult::Ok { duration_ms } => {
            let timeout_ms = (duration_ms as f64 * timeout_mult) as u64 + 2000;

            // In-place: write backup, mutate original, restore after
            let backup_content = source.to_string();
            let results = runner::run_mutations(
                abs_file,
                abs_test,
                source,
                mutations,
                resolved_cmd,
                working_dir,
                timeout_ms,
                mutation_args,
            );
            // run_mutations already restores original
            let _ = backup_content; // ensure we have the original

            finalize_results(&results, mutations, display_file, json_mode, quiet)
        }
    }
}

fn finalize_results(
    results: &[mutator::mutants::MutantResult],
    _mutations: &[mutator::mutants::Mutation],
    display_file: &std::path::Path,
    json_mode: bool,
    quiet: bool,
) -> i32 {
    let survived: Vec<_> = results
        .iter()
        .filter(|r| r.status == mutants::MutantStatus::Survived)
        .collect();
    let killed = results.iter().filter(|r| r.status == mutants::MutantStatus::Killed).count();
    let timed_out = results.iter().filter(|r| r.status == mutants::MutantStatus::Timeout).count();
    let unviable = results.iter().filter(|r| r.status == mutants::MutantStatus::Unviable).count();
    let total = results.len();
    let testable = total - unviable;
    let score = if testable > 0 {
        killed as f64 / testable as f64
    } else {
        1.0
    };

    let display_str = display_file.display().to_string();
    let survived_details: Vec<state::SurvivedMutant> = survived
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let m = &r.mutation;
            state::SurvivedMutant {
                ref_id: format!("m{}", i + 1),
                file: display_str.clone(),
                line: m.line,
                column: m.column,
                operator: m.operator.clone(),
                original: m.original.clone(),
                replacement: m.replacement.clone(),
                diff: r.diff.clone(),
                context_before: m.context_before.clone(),
                context_after: m.context_after.clone(),
            }
        })
        .collect();

    let run_result = state::RunResult {
        score,
        total,
        killed,
        survived: survived_details.len(),
        timeout: timed_out,
        unviable,
        duration_ms: results.iter().map(|r| r.duration_ms).sum(),
        survived_mutants: survived_details,
    };

    state::save_last_run(&run_result);

    if quiet {
        return if run_result.survived > 0 { 1 } else { 0 };
    }

    if json_mode {
        println!("{}", serde_json::to_string(&run_result).unwrap());
    } else {
        output::print_run_result(&run_result, display_file);
    }

    if run_result.survived > 0 { 1 } else { 0 }
}

fn cmd_show(mutant_ref: String, json_mode: bool) -> i32 {
    let ref_id = mutant_ref.trim_start_matches('@');

    let last_run = match state::load_last_run() {
        Some(r) => r,
        None => {
            output::print_error("No previous run found. Run `mutator run` first.");
            return 2;
        }
    };

    let mutant = last_run.survived_mutants.iter().find(|m| m.ref_id == ref_id);
    match mutant {
        Some(m) => {
            if json_mode {
                println!("{}", serde_json::to_string(m).unwrap());
            } else {
                output::print_mutant_detail(m);
            }
            0
        }
        None => {
            let valid: Vec<_> = last_run.survived_mutants.iter().map(|m| format!("@{}", m.ref_id)).collect();
            output::print_error(&format!(
                "Mutant @{} not found. Valid refs: {}",
                ref_id,
                valid.join(", ")
            ));
            2
        }
    }
}

fn cmd_status(json_mode: bool) -> i32 {
    match state::load_last_run() {
        Some(result) => {
            if json_mode {
                println!("{}", serde_json::to_string(&result).unwrap());
            } else {
                output::print_status(&result);
            }
            0
        }
        None => {
            output::print_error("No previous run found. Run `mutator run` first.");
            2
        }
    }
}
