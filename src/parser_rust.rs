use tree_sitter::{Node, Parser};
use crate::mutants::Mutation;

pub fn discover_mutations(source: &str, function_name: Option<&str>) -> Vec<Mutation> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser.set_language(&language.into()).expect("Failed to set Rust grammar");

    let tree = parser.parse(source, None).expect("Failed to parse Rust source");
    let root = tree.root_node();
    let lines: Vec<&str> = source.lines().collect();

    let mut mutations = Vec::new();

    match function_name {
        Some(name) => {
            if let Some(func_node) = find_function(root, name, source) {
                walk_node(func_node, source, &lines, &mut mutations);
            }
        }
        None => {
            collect_all_functions(root, source, &lines, &mut mutations);
        }
    }

    mutations
}

fn find_function<'a>(node: Node<'a>, name: &str, source: &str) -> Option<Node<'a>> {
    if node.kind() == "function_item" {
        if let Some(name_node) = node.child_by_field_name("name") {
            if node_text(name_node, source) == name {
                return Some(node);
            }
        }
    }
    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            if let Some(found) = find_function(child, name, source) {
                return Some(found);
            }
        }
    }
    None
}

fn collect_all_functions(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    if node.kind() == "function_item" {
        walk_node(node, source, lines, mutations);
        return;
    }
    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            collect_all_functions(child, source, lines, mutations);
        }
    }
}

pub fn list_functions(source: &str) -> Vec<String> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser.set_language(&language.into()).expect("Failed to set Rust grammar");

    let tree = parser.parse(source, None).expect("Failed to parse Rust source");
    let root = tree.root_node();
    let mut names = Vec::new();
    collect_function_names(root, source, &mut names);
    names
}

fn collect_function_names(node: Node, source: &str, names: &mut Vec<String>) {
    if node.kind() == "function_item" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            names.push(name.to_string());
        }
    }
    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            collect_function_names(child, source, names);
        }
    }
}

fn walk_node(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    if should_skip_node(node, source) {
        return;
    }

    match node.kind() {
        "binary_expression" => {
            collect_binary_mutations(node, source, lines, mutations);
        }
        "unary_expression" => {
            collect_unary_mutations(node, source, lines, mutations);
        }
        "return_expression" => {
            collect_return_mutations(node, source, lines, mutations);
        }
        "boolean_literal" => {
            collect_boolean_mutations(node, source, lines, mutations);
        }
        "if_expression" => {
            collect_if_body_mutations(node, source, lines, mutations);
        }
        _ => {}
    }

    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            walk_node(child, source, lines, mutations);
        }
    }
}

fn should_skip_node(node: Node, source: &str) -> bool {
    // Skip macro invocations (println!, eprintln!, log::, etc.)
    if node.kind() == "macro_invocation" {
        if let Some(mac) = node.child(0) {
            let text = node_text(mac, source);
            if text.starts_with("println")
                || text.starts_with("eprintln")
                || text.starts_with("print")
                || text.starts_with("log")
                || text.starts_with("debug")
                || text.starts_with("info")
                || text.starts_with("warn")
                || text.starts_with("error")
                || text.starts_with("trace")
                || text == "format"
            {
                return true;
            }
        }
    }
    false
}

fn get_context(lines: &[&str], line_idx: usize, range: usize) -> (Vec<String>, Vec<String>) {
    let start = line_idx.saturating_sub(range);
    let end = (line_idx + range + 1).min(lines.len());
    let before: Vec<String> = lines[start..line_idx].iter().map(|s| s.to_string()).collect();
    let after: Vec<String> = if line_idx + 1 < end {
        lines[line_idx + 1..end].iter().map(|s| s.to_string()).collect()
    } else {
        vec![]
    };
    (before, after)
}

fn node_text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

struct RustMutationOp {
    operator_name: &'static str,
    replacement: &'static str,
}

fn comparison_mutations(op: &str) -> Vec<RustMutationOp> {
    match op {
        ">" => vec![
            RustMutationOp { operator_name: "boundary", replacement: ">=" },
            RustMutationOp { operator_name: "negate_cmp", replacement: "<=" },
        ],
        ">=" => vec![
            RustMutationOp { operator_name: "boundary", replacement: ">" },
            RustMutationOp { operator_name: "negate_cmp", replacement: "<" },
        ],
        "<" => vec![
            RustMutationOp { operator_name: "boundary", replacement: "<=" },
            RustMutationOp { operator_name: "negate_cmp", replacement: ">=" },
        ],
        "<=" => vec![
            RustMutationOp { operator_name: "boundary", replacement: "<" },
            RustMutationOp { operator_name: "negate_cmp", replacement: ">" },
        ],
        "==" => vec![
            RustMutationOp { operator_name: "negate_eq", replacement: "!=" },
        ],
        "!=" => vec![
            RustMutationOp { operator_name: "negate_eq", replacement: "==" },
        ],
        _ => vec![],
    }
}

fn logical_mutations(op: &str) -> Vec<RustMutationOp> {
    match op {
        "&&" => vec![RustMutationOp { operator_name: "logic_flip", replacement: "||" }],
        "||" => vec![RustMutationOp { operator_name: "logic_flip", replacement: "&&" }],
        _ => vec![],
    }
}

fn arithmetic_mutations(op: &str) -> Vec<RustMutationOp> {
    match op {
        "+" => vec![RustMutationOp { operator_name: "arith", replacement: "-" }],
        "-" => vec![RustMutationOp { operator_name: "arith", replacement: "+" }],
        "*" => vec![RustMutationOp { operator_name: "arith", replacement: "/" }],
        "/" => vec![RustMutationOp { operator_name: "arith", replacement: "*" }],
        "%" => vec![RustMutationOp { operator_name: "arith", replacement: "/" }],
        _ => vec![],
    }
}

fn collect_binary_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    // binary_expression: left operator right
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            let kind = child.kind();
            let op_text = node_text(child, source);

            let ops: Vec<RustMutationOp> = match kind {
                ">" | ">=" | "<" | "<=" | "==" | "!=" => comparison_mutations(op_text),
                "&&" | "||" => logical_mutations(op_text),
                "+" | "-" | "*" | "/" | "%" => arithmetic_mutations(op_text),
                _ => vec![],
            };

            if ops.is_empty() {
                continue;
            }

            let line = child.start_position().row + 1;
            let col = child.start_position().column + 1;
            let (ctx_before, ctx_after) = get_context(lines, child.start_position().row, 2);

            for op in ops {
                mutations.push(Mutation {
                    line,
                    column: col,
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                    operator: op.operator_name.to_string(),
                    original: op_text.to_string(),
                    replacement: op.replacement.to_string(),
                    context_before: ctx_before.clone(),
                    context_after: ctx_after.clone(),
                });
            }
        }
    }
}

fn collect_unary_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    // unary_expression: ! operand
    if let Some(op_node) = node.child(0) {
        if op_node.kind() == "!" {
            if let Some(operand) = node.child(1) {
                let line = op_node.start_position().row + 1;
                let col = op_node.start_position().column + 1;
                let (ctx_before, ctx_after) = get_context(lines, op_node.start_position().row, 2);

                mutations.push(Mutation {
                    line,
                    column: col,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    operator: "negate_remove".to_string(),
                    original: node_text(node, source).to_string(),
                    replacement: node_text(operand, source).to_string(),
                    context_before: ctx_before,
                    context_after: ctx_after,
                });
            }
        }
    }
}

fn collect_return_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    // return_expression: "return" expr?
    // In Rust, the last expression in a block is an implicit return,
    // but explicit `return` statements are return_expression nodes.
    if node.child_count() < 2 {
        return;
    }
    if let Some(expr) = node.child(1) {
        let expr_text = node_text(expr, source).trim();
        let line = node.start_position().row + 1;
        let col = node.start_position().column + 1;
        let (ctx_before, ctx_after) = get_context(lines, node.start_position().row, 2);

        let replacement = if expr_text == "true" {
            "return false"
        } else if expr_text == "false" {
            "return true"
        } else if expr_text == "None" || expr_text == "()" {
            return; // No useful mutation for unit return
        } else if expr_text == "0" {
            "return 1"
        } else if expr_text.starts_with('"') {
            "return \"\".to_string()"
        } else if expr_text.starts_with("vec!") || expr_text.starts_with("Vec::") {
            "return vec![]"
        } else if expr_text == "Ok(())" {
            return;
        } else {
            "return Default::default()"
        };

        mutations.push(Mutation {
            line,
            column: col,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            operator: "return_val".to_string(),
            original: node_text(node, source).to_string(),
            replacement: replacement.to_string(),
            context_before: ctx_before,
            context_after: ctx_after,
        });
    }
}

fn collect_boolean_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    // Skip if inside a return (handled by return_mutations)
    if let Some(parent) = node.parent() {
        if parent.kind() == "return_expression" {
            return;
        }
    }

    let text = node_text(node, source);
    let line = node.start_position().row + 1;
    let col = node.start_position().column + 1;
    let (ctx_before, ctx_after) = get_context(lines, node.start_position().row, 2);

    let replacement = match text {
        "true" => "false",
        "false" => "true",
        _ => return,
    };

    mutations.push(Mutation {
        line,
        column: col,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        operator: "bool_flip".to_string(),
        original: text.to_string(),
        replacement: replacement.to_string(),
        context_before: ctx_before,
        context_after: ctx_after,
    });
}

fn collect_if_body_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    // if_expression: "if" condition consequence [else_clause]
    if let Some(consequence) = node.child_by_field_name("consequence") {
        if consequence.kind() == "block" {
            let block_text = node_text(consequence, source);
            if block_text.trim() == "{}" {
                return;
            }

            let line = consequence.start_position().row + 1;
            let col = consequence.start_position().column + 1;
            let (ctx_before, ctx_after) = get_context(lines, consequence.start_position().row, 2);

            mutations.push(Mutation {
                line,
                column: col,
                start_byte: consequence.start_byte(),
                end_byte: consequence.end_byte(),
                operator: "block_remove".to_string(),
                original: block_text.to_string(),
                replacement: "{}".to_string(),
                context_before: ctx_before,
                context_after: ctx_after,
            });
        }
    }
}
