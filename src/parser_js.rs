use tree_sitter::{Node, Parser};
use crate::mutants::Mutation;

#[derive(Clone, Copy)]
pub enum JsDialect {
    JavaScript,
    TypeScript,
    Tsx,
}

pub fn discover_mutations(source: &str, function_name: Option<&str>, dialect: JsDialect) -> Vec<Mutation> {
    let mut parser = Parser::new();
    let language = match dialect {
        JsDialect::JavaScript => tree_sitter_javascript::LANGUAGE,
        JsDialect::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        JsDialect::Tsx => tree_sitter_typescript::LANGUAGE_TSX,
    };
    parser.set_language(&language.into()).expect("Failed to set JS/TS grammar");

    let tree = parser.parse(source, None).expect("Failed to parse JS/TS source");
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

pub fn list_functions(source: &str, dialect: JsDialect) -> Vec<String> {
    let mut parser = Parser::new();
    let language = match dialect {
        JsDialect::JavaScript => tree_sitter_javascript::LANGUAGE,
        JsDialect::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        JsDialect::Tsx => tree_sitter_typescript::LANGUAGE_TSX,
    };
    parser.set_language(&language.into()).expect("Failed to set JS/TS grammar");

    let tree = parser.parse(source, None).expect("Failed to parse JS/TS source");
    let root = tree.root_node();
    let mut names = Vec::new();
    collect_function_names(root, source, &mut names);
    names
}

fn find_function<'a>(node: Node<'a>, name: &str, source: &str) -> Option<Node<'a>> {
    match node.kind() {
        // function foo() {}
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if node_text(name_node, source) == name {
                    return Some(node);
                }
            }
        }
        // class { foo() {} }
        "method_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if node_text(name_node, source) == name {
                    return Some(node);
                }
            }
        }
        // const foo = () => {} or const foo = function() {}
        "lexical_declaration" | "variable_declaration" => {
            let count = node.child_count();
            for i in 0..count {
                if let Some(declarator) = node.child(i) {
                    if declarator.kind() == "variable_declarator" {
                        if let Some(name_node) = declarator.child_by_field_name("name") {
                            if node_text(name_node, source) == name {
                                if let Some(value) = declarator.child_by_field_name("value") {
                                    if is_function_node(value.kind()) {
                                        return Some(node);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // export function foo() {} or export default function foo() {}
        "export_statement" => {
            let count = node.child_count();
            for i in 0..count {
                if let Some(child) = node.child(i) {
                    if let Some(found) = find_function(child, name, source) {
                        return Some(found);
                    }
                }
            }
        }
        _ => {}
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

fn is_function_node(kind: &str) -> bool {
    matches!(kind, "arrow_function" | "function" | "generator_function")
}

fn collect_all_functions(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" | "method_definition" => {
            walk_node(node, source, lines, mutations);
            return;
        }
        "lexical_declaration" | "variable_declaration" => {
            let count = node.child_count();
            for i in 0..count {
                if let Some(declarator) = node.child(i) {
                    if declarator.kind() == "variable_declarator" {
                        if let Some(value) = declarator.child_by_field_name("value") {
                            if is_function_node(value.kind()) {
                                walk_node(value, source, lines, mutations);
                                return;
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    let count = node.child_count();
    for i in 0..count {
        if let Some(child) = node.child(i) {
            collect_all_functions(child, source, lines, mutations);
        }
    }
}

fn collect_function_names(node: Node, source: &str, names: &mut Vec<String>) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(name_node, source);
                if !name.starts_with("test") && !name.starts_with("_") {
                    names.push(name.to_string());
                }
            }
        }
        "method_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(name_node, source);
                if !name.starts_with("test") && name != "constructor" {
                    names.push(name.to_string());
                }
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            let count = node.child_count();
            for i in 0..count {
                if let Some(declarator) = node.child(i) {
                    if declarator.kind() == "variable_declarator" {
                        if let Some(value) = declarator.child_by_field_name("value") {
                            if is_function_node(value.kind()) {
                                if let Some(name_node) = declarator.child_by_field_name("name") {
                                    let name = node_text(name_node, source);
                                    if !name.starts_with("test") && !name.starts_with("_") {
                                        names.push(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
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
        "return_statement" => {
            collect_return_mutations(node, source, lines, mutations);
        }
        "true" | "false" => {
            collect_boolean_mutations(node, source, lines, mutations);
        }
        "if_statement" => {
            collect_if_body_mutations(node, source, lines, mutations);
        }
        "for_statement" | "for_in_statement" | "while_statement" => {
            collect_loop_body_mutations(node, source, lines, mutations);
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
    if node.kind() == "call_expression" {
        if let Some(func) = node.child_by_field_name("function") {
            let text = node_text(func, source);
            if text == "console.log"
                || text == "console.warn"
                || text == "console.error"
                || text == "console.info"
                || text == "console.debug"
            {
                return true;
            }
        }
    }
    // Skip string expression statements (like 'use strict')
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

struct JsMutationOp {
    operator_name: &'static str,
    replacement: &'static str,
}

fn comparison_mutations(op: &str) -> Vec<JsMutationOp> {
    match op {
        ">" => vec![
            JsMutationOp { operator_name: "boundary", replacement: ">=" },
            JsMutationOp { operator_name: "negate_cmp", replacement: "<=" },
        ],
        ">=" => vec![
            JsMutationOp { operator_name: "boundary", replacement: ">" },
            JsMutationOp { operator_name: "negate_cmp", replacement: "<" },
        ],
        "<" => vec![
            JsMutationOp { operator_name: "boundary", replacement: "<=" },
            JsMutationOp { operator_name: "negate_cmp", replacement: ">=" },
        ],
        "<=" => vec![
            JsMutationOp { operator_name: "boundary", replacement: "<" },
            JsMutationOp { operator_name: "negate_cmp", replacement: ">" },
        ],
        "==" => vec![
            JsMutationOp { operator_name: "negate_eq", replacement: "!=" },
        ],
        "!=" => vec![
            JsMutationOp { operator_name: "negate_eq", replacement: "==" },
        ],
        "===" => vec![
            JsMutationOp { operator_name: "negate_eq", replacement: "!==" },
        ],
        "!==" => vec![
            JsMutationOp { operator_name: "negate_eq", replacement: "===" },
        ],
        _ => vec![],
    }
}

fn logical_mutations(op: &str) -> Vec<JsMutationOp> {
    match op {
        "&&" => vec![JsMutationOp { operator_name: "logic_flip", replacement: "||" }],
        "||" => vec![JsMutationOp { operator_name: "logic_flip", replacement: "&&" }],
        "??" => vec![JsMutationOp { operator_name: "logic_flip", replacement: "||" }],
        _ => vec![],
    }
}

fn arithmetic_mutations(op: &str) -> Vec<JsMutationOp> {
    match op {
        "+" => vec![JsMutationOp { operator_name: "arith", replacement: "-" }],
        "-" => vec![JsMutationOp { operator_name: "arith", replacement: "+" }],
        "*" => vec![JsMutationOp { operator_name: "arith", replacement: "/" }],
        "/" => vec![JsMutationOp { operator_name: "arith", replacement: "*" }],
        "%" => vec![JsMutationOp { operator_name: "arith", replacement: "/" }],
        "**" => vec![JsMutationOp { operator_name: "arith", replacement: "*" }],
        _ => vec![],
    }
}

fn collect_binary_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    if let Some(op_node) = node.child_by_field_name("operator") {
        let op_text = node_text(op_node, source);

        let ops: Vec<JsMutationOp> = match op_text {
            ">" | ">=" | "<" | "<=" | "==" | "!=" | "===" | "!==" => comparison_mutations(op_text),
            "&&" | "||" | "??" => logical_mutations(op_text),
            "+" | "-" | "*" | "/" | "%" | "**" => {
                // Skip string concatenation
                if op_text == "+" {
                    if let Some(left) = node.child_by_field_name("left") {
                        if left.kind() == "string" || left.kind() == "template_string" {
                            return;
                        }
                    }
                }
                arithmetic_mutations(op_text)
            }
            _ => vec![],
        };

        if ops.is_empty() {
            return;
        }

        let line = op_node.start_position().row + 1;
        let col = op_node.start_position().column + 1;
        let (ctx_before, ctx_after) = get_context(lines, op_node.start_position().row, 2);

        for op in ops {
            mutations.push(Mutation {
                line,
                column: col,
                start_byte: op_node.start_byte(),
                end_byte: op_node.end_byte(),
                operator: op.operator_name.to_string(),
                original: op_text.to_string(),
                replacement: op.replacement.to_string(),
                context_before: ctx_before.clone(),
                context_after: ctx_after.clone(),
            });
        }
    }
}

fn collect_unary_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    if let Some(op_node) = node.child_by_field_name("operator") {
        if op_node.kind() == "!" {
            if let Some(operand) = node.child_by_field_name("argument") {
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
    // return_statement children: "return" [expression] [";"]
    let mut expr = None;
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            if child.kind() != "return" && child.kind() != ";" {
                expr = Some(child);
                break;
            }
        }
    }

    let line = node.start_position().row + 1;
    let col = node.start_position().column + 1;
    let (ctx_before, ctx_after) = get_context(lines, node.start_position().row, 2);

    let expr = match expr {
        Some(e) => e,
        None => {
            // bare `return;`
            mutations.push(Mutation {
                line,
                column: col,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                operator: "return_val".to_string(),
                original: node_text(node, source).to_string(),
                replacement: "return undefined;".to_string(),
                context_before: ctx_before,
                context_after: ctx_after,
            });
            return;
        }
    };

    let expr_text = node_text(expr, source).trim();

    let replacement = if expr_text == "true" {
        "return false;"
    } else if expr_text == "false" {
        "return true;"
    } else if expr_text == "null" || expr_text == "undefined" {
        "return \"\";"
    } else if expr_text == "0" {
        "return 1;"
    } else if expr_text.starts_with('"') || expr_text.starts_with('\'') || expr_text.starts_with('`') {
        "return \"\";"
    } else if expr_text.starts_with('[') {
        "return [];"
    } else if expr_text == "{}" {
        "return null;"
    } else if expr_text.starts_with('{') {
        "return {};"
    } else if expr_text.parse::<f64>().is_ok() {
        "return 0;"
    } else {
        "return null;"
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

fn collect_boolean_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    // Skip if inside a return (handled by return_mutations)
    if let Some(parent) = node.parent() {
        if parent.kind() == "return_statement" {
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
    // if_statement has: condition, consequence (statement_block), alternative (else_clause)
    if let Some(consequence) = node.child_by_field_name("consequence") {
        if consequence.kind() == "statement_block" {
            add_block_remove_mutation(consequence, source, lines, mutations);
        }
    }

    if let Some(alternative) = node.child_by_field_name("alternative") {
        // else_clause contains either a statement_block or another if_statement (else if)
        if alternative.kind() == "else_clause" {
            let count = alternative.child_count();
            for i in 0..count {
                if let Some(child) = alternative.child(i) {
                    if child.kind() == "statement_block" {
                        add_block_remove_mutation(child, source, lines, mutations);
                    }
                    // else if is a nested if_statement, handled by recursion in walk_node
                }
            }
        }
    }
}

fn collect_loop_body_mutations(node: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    if let Some(body) = node.child_by_field_name("body") {
        if body.kind() == "statement_block" {
            add_block_remove_mutation(body, source, lines, mutations);
        }
    }
}

fn add_block_remove_mutation(block: Node, source: &str, lines: &[&str], mutations: &mut Vec<Mutation>) {
    let block_text = node_text(block, source);
    if block_text.trim() == "{}" {
        return;
    }

    let line = block.start_position().row + 1;
    let col = block.start_position().column + 1;
    let (ctx_before, ctx_after) = get_context(lines, block.start_position().row, 2);

    mutations.push(Mutation {
        line,
        column: col,
        start_byte: block.start_byte(),
        end_byte: block.end_byte(),
        operator: "block_remove".to_string(),
        original: block_text.to_string(),
        replacement: "{}".to_string(),
        context_before: ctx_before,
        context_after: ctx_after,
    });
}
