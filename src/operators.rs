/// Mutation operator definitions for Python.
/// Returns (original_pattern, replacement) pairs for a given AST node kind.

pub struct MutationOp {
    pub operator_name: &'static str,
    pub replacement: &'static str,
}

/// Tier 1: High-value operators (catch real bugs)
pub fn comparison_mutations(op_text: &str) -> Vec<MutationOp> {
    match op_text {
        ">" => vec![
            MutationOp { operator_name: "boundary", replacement: ">=" },
            MutationOp { operator_name: "negate_cmp", replacement: "<=" },
        ],
        ">=" => vec![
            MutationOp { operator_name: "boundary", replacement: ">" },
            MutationOp { operator_name: "negate_cmp", replacement: "<" },
        ],
        "<" => vec![
            MutationOp { operator_name: "boundary", replacement: "<=" },
            MutationOp { operator_name: "negate_cmp", replacement: ">=" },
        ],
        "<=" => vec![
            MutationOp { operator_name: "boundary", replacement: "<" },
            MutationOp { operator_name: "negate_cmp", replacement: ">" },
        ],
        "==" => vec![
            MutationOp { operator_name: "negate_eq", replacement: "!=" },
        ],
        "!=" => vec![
            MutationOp { operator_name: "negate_eq", replacement: "==" },
        ],
        "is" => vec![
            MutationOp { operator_name: "negate_is", replacement: "is not" },
        ],
        "is not" => vec![
            MutationOp { operator_name: "negate_is", replacement: "is" },
        ],
        "in" => vec![
            MutationOp { operator_name: "negate_in", replacement: "not in" },
        ],
        "not in" => vec![
            MutationOp { operator_name: "negate_in", replacement: "in" },
        ],
        _ => vec![],
    }
}

/// Tier 1: Boolean mutations
pub fn boolean_mutations(text: &str) -> Vec<MutationOp> {
    match text {
        "True" => vec![MutationOp { operator_name: "bool_flip", replacement: "False" }],
        "False" => vec![MutationOp { operator_name: "bool_flip", replacement: "True" }],
        _ => vec![],
    }
}

/// Tier 1: Logical operator mutations
pub fn logical_mutations(op_text: &str) -> Vec<MutationOp> {
    match op_text {
        "and" => vec![MutationOp { operator_name: "logic_flip", replacement: "or" }],
        "or" => vec![MutationOp { operator_name: "logic_flip", replacement: "and" }],
        "not" => vec![MutationOp { operator_name: "negate_remove", replacement: "" }],
        _ => vec![],
    }
}

/// Tier 1: Return value mutations
pub fn return_mutations(return_value: &str) -> Vec<MutationOp> {
    let trimmed = return_value.trim();
    let mut ops = vec![];

    if trimmed == "None" {
        ops.push(MutationOp { operator_name: "return_val", replacement: "return \"\"" });
    } else if trimmed == "True" {
        ops.push(MutationOp { operator_name: "return_val", replacement: "return False" });
    } else if trimmed == "False" {
        ops.push(MutationOp { operator_name: "return_val", replacement: "return True" });
    } else if trimmed.starts_with('"') || trimmed.starts_with('\'') || trimmed.starts_with("f\"") || trimmed.starts_with("f'") {
        ops.push(MutationOp { operator_name: "return_val", replacement: "return \"\"" });
    } else if trimmed == "[]" || trimmed.starts_with('[') {
        ops.push(MutationOp { operator_name: "return_val", replacement: "return []" });
    } else if trimmed == "{}" || trimmed.starts_with('{') {
        ops.push(MutationOp { operator_name: "return_val", replacement: "return {}" });
    } else if trimmed == "0" {
        ops.push(MutationOp { operator_name: "return_val", replacement: "return 1" });
    } else if trimmed.parse::<f64>().is_ok() {
        ops.push(MutationOp { operator_name: "return_val", replacement: "return 0" });
    } else {
        // Generic: return None for any other expression
        ops.push(MutationOp { operator_name: "return_val", replacement: "return None" });
    }

    ops
}

/// Tier 2: Arithmetic operator mutations
pub fn arithmetic_mutations(op_text: &str) -> Vec<MutationOp> {
    match op_text {
        "+" => vec![MutationOp { operator_name: "arith", replacement: "-" }],
        "-" => vec![MutationOp { operator_name: "arith", replacement: "+" }],
        "*" => vec![MutationOp { operator_name: "arith", replacement: "/" }],
        "/" => vec![MutationOp { operator_name: "arith", replacement: "*" }],
        "//" => vec![MutationOp { operator_name: "arith", replacement: "/" }],
        "%" => vec![MutationOp { operator_name: "arith", replacement: "/" }],
        "**" => vec![MutationOp { operator_name: "arith", replacement: "*" }],
        _ => vec![],
    }
}

/// Tier 2: String literal mutations
pub fn string_mutations(text: &str) -> Vec<MutationOp> {
    if text == "\"\"" || text == "''" {
        vec![MutationOp { operator_name: "string_mut", replacement: "\"mutator_xx\"" }]
    } else {
        vec![MutationOp { operator_name: "string_mut", replacement: "\"\"" }]
    }
}

/// Tier 1: Conditional body removal (if block -> pass)
pub fn conditional_body_removal() -> Vec<MutationOp> {
    vec![MutationOp { operator_name: "block_remove", replacement: "pass" }]
}
