use console::Style;
use crate::state::{RunResult, SurvivedMutant};
use std::path::Path;

pub fn print_error(msg: &str) {
    let style = Style::new().red().bold();
    eprintln!("{} {}", style.apply_to("✗"), msg);
}

pub fn print_success(msg: &str) {
    let style = Style::new().green().bold();
    println!("{} {}", style.apply_to("✓"), msg);
}

pub fn print_run_result(result: &RunResult, file: &Path) {
    let score_pct = result.score * 100.0;
    let testable = result.total - result.unviable;

    if result.survived == 0 {
        let style = Style::new().green().bold();
        println!(
            "{} {}: {} mutants, all killed ({:.1}%) in {:.1}s",
            style.apply_to("✓"),
            file.display(),
            testable,
            score_pct,
            result.duration_ms as f64 / 1000.0,
        );
        return;
    }

    let style = Style::new().yellow().bold();
    println!(
        "{} {}: {} survived / {} testable ({:.1}% killed) in {:.1}s",
        style.apply_to("!"),
        file.display(),
        result.survived,
        testable,
        score_pct,
        result.duration_ms as f64 / 1000.0,
    );

    if result.unviable > 0 {
        let dim = Style::new().dim();
        println!("  {} {} unviable mutants skipped", dim.apply_to("·"), result.unviable);
    }
    if result.timeout > 0 {
        let dim = Style::new().dim();
        println!("  {} {} mutants timed out", dim.apply_to("·"), result.timeout);
    }

    println!();
    for m in &result.survived_mutants {
        let ref_style = Style::new().cyan().bold();
        let loc_style = Style::new().dim();
        let op_style = Style::new().magenta();

        println!(
            "  {} {}:{} {} {} → {}",
            ref_style.apply_to(format!("@{}", m.ref_id)),
            m.file,
            m.line,
            loc_style.apply_to(format!("[{}]", m.operator)),
            op_style.apply_to(&m.original),
            op_style.apply_to(&m.replacement),
        );
    }
}

pub fn print_mutant_detail(m: &SurvivedMutant) {
    let ref_style = Style::new().cyan().bold();
    let dim = Style::new().dim();

    println!(
        "{} {}:{} [{}]",
        ref_style.apply_to(format!("@{}", m.ref_id)),
        m.file,
        m.line,
        m.operator,
    );
    println!();

    // Show context with the diff
    for line in &m.context_before {
        println!("  {}", dim.apply_to(line));
    }

    // Show the diff lines
    for line in m.diff.lines() {
        if line.starts_with('-') {
            let del_style = Style::new().red();
            println!("  {}", del_style.apply_to(line));
        } else if line.starts_with('+') {
            let add_style = Style::new().green();
            println!("  {}", add_style.apply_to(line));
        }
    }

    for line in &m.context_after {
        println!("  {}", dim.apply_to(line));
    }
}

pub fn print_status(result: &RunResult) {
    let score_pct = result.score * 100.0;
    let testable = result.total - result.unviable;

    println!(
        "Last run: {} mutants, {} killed, {} survived ({:.1}% score)",
        testable, result.killed, result.survived, score_pct,
    );

    if result.survived > 0 {
        println!();
        for m in &result.survived_mutants {
            let ref_style = Style::new().cyan().bold();
            println!(
                "  {} {}:{} {} → {}",
                ref_style.apply_to(format!("@{}", m.ref_id)),
                m.file,
                m.line,
                m.original,
                m.replacement,
            );
        }
        println!();
        println!("Use `mutator show @m1` for details on a specific mutant.");
    }
}
