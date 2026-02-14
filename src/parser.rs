use tree_sitter::{Node, Parser};
use crate::mutants::Mutation;
use crate::operators;

pub fn discover_mutations(source: &str, function_name: Option<&str>) -> Vec<Mutation> {
    let mut parser = Parser::new();
    let language = tree_sitter_python::LANGUAGE;
    parser.set_language(&language.into()).expect("Failed to set Python grammar");

    let tree = parser.parse(source, None).expect("Failed to parse source");
    let root = tree.root_node();
    let lines: Vec<&str> = source.lines().collect();

    let mut mutations = Vec::new();

    match function_name {
        Some(name) => {
            // Find the named function and only mutate within its body
            if let Some(func_node) = find_function(root, name, source) {
                walk_node(func_node, source, &lines, &mut mutations);
            }
        }
        None => {
            // Mutate all functions (skip module-level code)
            collect_all_functions(root, source, &lines, &mut mutations);
        }
    }

    mutations
}

/// Find a function_definition node by name.
fn find_function<'a>(node: Node<'a>, name: &str, source: &str) -> Option<Node<'a>> {
    if node.kind() == "function_definition" {
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

/// Collect mutations from all function bodies (skip module-level code).
fn collect_all_functions(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    if node.kind() == "function_definition" {
        walk_node(node, source, lines, mutations);
        return; // Don't recurse into nested functions twice
    }
    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            collect_all_functions(child, source, lines, mutations);
        }
    }
}

/// List all function names in the source file.
pub fn list_functions(source: &str) -> Vec<String> {
    let mut parser = Parser::new();
    let language = tree_sitter_python::LANGUAGE;
    parser.set_language(&language.into()).expect("Failed to set Python grammar");

    let tree = parser.parse(source, None).expect("Failed to parse source");
    let root = tree.root_node();
    let mut names = Vec::new();
    collect_function_names(root, source, &mut names);
    names
}

fn collect_function_names(node: Node, source: &str, names: &mut Vec<String>) {
    if node.kind() == "function_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = node_text(name_node, source);
            // Skip dunder methods and test functions
            if !name.starts_with("__") && !name.starts_with("test_") {
                names.push(name.to_string());
            }
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
    // Skip nodes that are noise for business logic testing
    if should_skip_node(node, source) {
        return;
    }

    match node.kind() {
        "comparison_operator" => {
            collect_comparison_mutations(node, source, lines, mutations);
        }
        "boolean_operator" => {
            collect_boolean_operator_mutations(node, source, lines, mutations);
        }
        "not_operator" => {
            collect_not_operator_mutations(node, source, lines, mutations);
        }
        "binary_operator" => {
            collect_arithmetic_mutations(node, source, lines, mutations);
        }
        "return_statement" => {
            collect_return_mutations(node, source, lines, mutations);
        }
        "true" | "false" => {
            collect_boolean_literal_mutations(node, source, lines, mutations);
        }
        "if_statement" => {
            collect_if_body_mutations(node, source, lines, mutations);
        }
        // String mutations deliberately excluded from defaults.
        // They mostly test formatting, not business logic.
        _ => {}
    }

    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            walk_node(child, source, lines, mutations);
        }
    }
}

/// Skip nodes that are not business logic: print calls, logging,
/// string literals used as dict keys or format strings in print/log.
fn should_skip_node(node: Node, source: &str) -> bool {
    // Skip entire call expressions that are print/logging
    if node.kind() == "call" {
        if let Some(func) = node.child(0) {
            let text = node_text(func, source);
            if text == "print"
                || text == "logging.info"
                || text == "logging.debug"
                || text == "logging.warning"
                || text == "logging.error"
                || text.starts_with("log.")
            {
                return true;
            }
        }
    }
    // Skip expression_statement that is just a string (docstring)
    if node.kind() == "expression_statement" && node.child_count() == 1 {
        if let Some(child) = node.child(0) {
            if child.kind() == "string" {
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

fn collect_comparison_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            let kind = child.kind();
            let op_str = match kind {
                "<" | ">" | "<=" | ">=" | "==" | "!=" | "is" | "in" => {
                    node_text(child, source).to_string()
                }
                "is not" | "not in" => {
                    node_text(child, source).to_string()
                }
                _ => continue,
            };

            let line = child.start_position().row + 1;
            let col = child.start_position().column + 1;
            let (ctx_before, ctx_after) = get_context(lines, child.start_position().row, 2);

            for op in operators::comparison_mutations(&op_str) {
                mutations.push(Mutation {
                    line,
                    column: col,
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                    operator: op.operator_name.to_string(),
                    original: op_str.clone(),
                    replacement: op.replacement.to_string(),
                    context_before: ctx_before.clone(),
                    context_after: ctx_after.clone(),
                });
            }
        }
    }
}

fn collect_boolean_operator_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            if child.kind() == "and" || child.kind() == "or" {
                let op_text = node_text(child, source);
                let line = child.start_position().row + 1;
                let col = child.start_position().column + 1;
                let (ctx_before, ctx_after) = get_context(lines, child.start_position().row, 2);

                for op in operators::logical_mutations(op_text) {
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
}

fn collect_not_operator_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    if let Some(not_kw) = node.child(0) {
        if not_kw.kind() == "not" {
            if let Some(operand) = node.child(1) {
                let line = not_kw.start_position().row + 1;
                let col = not_kw.start_position().column + 1;
                let (ctx_before, ctx_after) = get_context(lines, not_kw.start_position().row, 2);
                let operand_text = node_text(operand, source);

                mutations.push(Mutation {
                    line,
                    column: col,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    operator: "negate_remove".to_string(),
                    original: node_text(node, source).to_string(),
                    replacement: operand_text.to_string(),
                    context_before: ctx_before,
                    context_after: ctx_after,
                });
            }
        }
    }
}

fn collect_arithmetic_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            let kind = child.kind();
            if kind == "+" || kind == "-" || kind == "*" || kind == "/"
                || kind == "//" || kind == "%" || kind == "**"
            {
                let op_text = node_text(child, source);
                let line = child.start_position().row + 1;
                let col = child.start_position().column + 1;
                let (ctx_before, ctx_after) = get_context(lines, child.start_position().row, 2);

                // Skip string concatenation
                if kind == "+" {
                    if let Some(left) = node.child(0) {
                        if left.kind() == "string" || left.kind() == "concatenated_string" {
                            continue;
                        }
                    }
                }

                for op in operators::arithmetic_mutations(op_text) {
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
}

fn collect_return_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    let child_count = node.child_count();
    if child_count < 2 {
        let line = node.start_position().row + 1;
        let col = node.start_position().column + 1;
        let (ctx_before, ctx_after) = get_context(lines, node.start_position().row, 2);
        mutations.push(Mutation {
            line,
            column: col,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            operator: "return_val".to_string(),
            original: node_text(node, source).to_string(),
            replacement: "return None".to_string(),
            context_before: ctx_before,
            context_after: ctx_after,
        });
        return;
    }

    if let Some(expr) = node.child(1) {
        let expr_text = node_text(expr, source);
        let line = node.start_position().row + 1;
        let col = node.start_position().column + 1;
        let (ctx_before, ctx_after) = get_context(lines, node.start_position().row, 2);

        for op in operators::return_mutations(expr_text) {
            mutations.push(Mutation {
                line,
                column: col,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                operator: op.operator_name.to_string(),
                original: node_text(node, source).to_string(),
                replacement: op.replacement.to_string(),
                context_before: ctx_before.clone(),
                context_after: ctx_after.clone(),
            });
        }
    }
}

fn collect_boolean_literal_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    if let Some(parent) = node.parent() {
        if parent.kind() == "return_statement" {
            return;
        }
    }

    let text = node_text(node, source);
    let line = node.start_position().row + 1;
    let col = node.start_position().column + 1;
    let (ctx_before, ctx_after) = get_context(lines, node.start_position().row, 2);

    for op in operators::boolean_mutations(text) {
        mutations.push(Mutation {
            line,
            column: col,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            operator: op.operator_name.to_string(),
            original: text.to_string(),
            replacement: op.replacement.to_string(),
            context_before: ctx_before.clone(),
            context_after: ctx_after.clone(),
        });
    }
}

fn collect_if_body_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            if child.kind() == "block" {
                let block_text = node_text(child, source);
                if block_text.trim() == "pass" {
                    continue;
                }

                let line = child.start_position().row + 1;
                let col = child.start_position().column + 1;
                let (ctx_before, ctx_after) = get_context(lines, child.start_position().row, 2);

                let indent = " ".repeat(child.start_position().column);
                let replacement = format!("\n{}pass", indent);

                mutations.push(Mutation {
                    line,
                    column: col,
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                    operator: "block_remove".to_string(),
                    original: block_text.to_string(),
                    replacement,
                    context_before: ctx_before,
                    context_after: ctx_after,
                });

                break;
            }
        }
    }
}
