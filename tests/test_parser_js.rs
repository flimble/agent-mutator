use mutator::parser_js::{self, JsDialect};

fn js_mutations(source: &str, func: Option<&str>) -> Vec<mutator::mutants::Mutation> {
    parser_js::discover_mutations(source, func, JsDialect::JavaScript)
}

fn ts_mutations(source: &str, func: Option<&str>) -> Vec<mutator::mutants::Mutation> {
    parser_js::discover_mutations(source, func, JsDialect::TypeScript)
}

fn tsx_mutations(source: &str, func: Option<&str>) -> Vec<mutator::mutants::Mutation> {
    parser_js::discover_mutations(source, func, JsDialect::Tsx)
}

// --- Function discovery ---

#[test]
fn discovers_function_declaration() {
    let source = r#"
function check(x) {
    if (x > 0) {
        return true;
    }
    return false;
}
"#;
    let mutations = js_mutations(source, Some("check"));
    assert!(!mutations.is_empty(), "Should find mutations in function declaration");
    let comparisons: Vec<_> = mutations.iter().filter(|m| m.operator == "boundary" || m.operator == "negate_cmp").collect();
    assert!(comparisons.len() >= 2, "Expected comparison mutations, got {}", comparisons.len());
}

#[test]
fn discovers_arrow_function() {
    let source = r#"
const check = (x) => {
    return x > 0;
};
"#;
    let mutations = js_mutations(source, Some("check"));
    assert!(!mutations.is_empty(), "Should find mutations in arrow function");
}

#[test]
fn discovers_method_definition() {
    let source = r#"
class Foo {
    check(x) {
        return x > 0;
    }
}
"#;
    let mutations = js_mutations(source, Some("check"));
    assert!(!mutations.is_empty(), "Should find mutations in method definition");
}

#[test]
fn discovers_exported_function() {
    let source = r#"
export function check(x) {
    return x > 0;
}
"#;
    let mutations = ts_mutations(source, Some("check"));
    assert!(!mutations.is_empty(), "Should find mutations in exported function");
}

// --- Comparison operators ---

#[test]
fn comparison_gt() {
    let source = "function f(x) { return x > 0; }";
    let mutations = js_mutations(source, Some("f"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == ">").collect();
    assert_eq!(cmp.len(), 2); // boundary + negate
    assert!(cmp.iter().any(|m| m.replacement == ">="));
    assert!(cmp.iter().any(|m| m.replacement == "<="));
}

#[test]
fn comparison_strict_equals() {
    let source = "function f(x) { return x === 0; }";
    let mutations = js_mutations(source, Some("f"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == "===").collect();
    assert_eq!(cmp.len(), 1);
    assert_eq!(cmp[0].replacement, "!==");
}

#[test]
fn comparison_strict_not_equals() {
    let source = "function f(x) { return x !== 0; }";
    let mutations = js_mutations(source, Some("f"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == "!==").collect();
    assert_eq!(cmp.len(), 1);
    assert_eq!(cmp[0].replacement, "===");
}

#[test]
fn comparison_loose_equals() {
    let source = "function f(x) { return x == 0; }";
    let mutations = js_mutations(source, Some("f"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == "==").collect();
    assert_eq!(cmp.len(), 1);
    assert_eq!(cmp[0].replacement, "!=");
}

// --- Logical operators ---

#[test]
fn logical_and_to_or() {
    let source = "function f(a, b) { return a && b; }";
    let mutations = js_mutations(source, Some("f"));
    let logic: Vec<_> = mutations.iter().filter(|m| m.operator == "logic_flip").collect();
    assert_eq!(logic.len(), 1);
    assert_eq!(logic[0].original, "&&");
    assert_eq!(logic[0].replacement, "||");
}

#[test]
fn logical_or_to_and() {
    let source = "function f(a, b) { return a || b; }";
    let mutations = js_mutations(source, Some("f"));
    let logic: Vec<_> = mutations.iter().filter(|m| m.operator == "logic_flip").collect();
    assert_eq!(logic.len(), 1);
    assert_eq!(logic[0].replacement, "&&");
}

#[test]
fn nullish_coalescing_to_or() {
    let source = "function f(a, b) { return a ?? b; }";
    let mutations = js_mutations(source, Some("f"));
    let logic: Vec<_> = mutations.iter().filter(|m| m.original == "??").collect();
    assert_eq!(logic.len(), 1);
    assert_eq!(logic[0].replacement, "||");
}

// --- Negation removal ---

#[test]
fn negation_removal() {
    let source = "function f(x) { if (!x) { return 1; } return 0; }";
    let mutations = js_mutations(source, Some("f"));
    let nots: Vec<_> = mutations.iter().filter(|m| m.operator == "negate_remove").collect();
    assert_eq!(nots.len(), 1);
}

// --- Arithmetic ---

#[test]
fn arithmetic_plus_to_minus() {
    let source = "function add(a, b) { return a + b; }";
    let mutations = js_mutations(source, Some("add"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].original, "+");
    assert_eq!(arith[0].replacement, "-");
}

#[test]
fn skips_string_concatenation() {
    let source = r#"function f() { return "hello" + " world"; }"#;
    let mutations = js_mutations(source, Some("f"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 0, "Should not mutate string concatenation");
}

// --- Boolean literals ---

#[test]
fn boolean_true_flips() {
    let source = "function f() { let x = true; return x; }";
    let mutations = js_mutations(source, Some("f"));
    let bools: Vec<_> = mutations.iter().filter(|m| m.operator == "bool_flip").collect();
    assert_eq!(bools.len(), 1);
    assert_eq!(bools[0].original, "true");
    assert_eq!(bools[0].replacement, "false");
}

// --- Return mutations ---

#[test]
fn return_true_becomes_false() {
    let source = "function f() { return true; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("false"));
}

#[test]
fn return_string_becomes_empty() {
    let source = r#"function f() { return "hello"; }"#;
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("\"\""));
}

#[test]
fn return_null_becomes_empty_string() {
    let source = "function f() { return null; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("\"\""));
}

#[test]
fn return_number_becomes_zero() {
    let source = "function f() { return 42; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("0"));
}

#[test]
fn return_zero_becomes_one() {
    let source = "function f() { return 0; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("1"));
}

#[test]
fn return_empty_object_becomes_null() {
    let source = "function f() { return {}; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("null"), "return {{}} should become return null, got: {}", rets[0].replacement);
}

// --- Block removal ---

#[test]
fn if_body_block_remove() {
    let source = r#"
function f(x) {
    if (x > 0) {
        x = x + 1;
        return x;
    }
    return 0;
}
"#;
    let mutations = js_mutations(source, Some("f"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.len() >= 1, "Should create block_remove mutation for if body");
    assert_eq!(blocks[0].replacement, "{}");
}

#[test]
fn else_body_block_remove() {
    let source = r#"
function f(x) {
    if (x > 0) {
        return 1;
    } else {
        return 0;
    }
}
"#;
    let mutations = js_mutations(source, Some("f"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert_eq!(blocks.len(), 2, "Should create block_remove for both if and else bodies");
}

#[test]
fn else_if_body_block_remove() {
    let source = r#"
function f(x) {
    if (x > 0) {
        return 1;
    } else if (x < 0) {
        return -1;
    } else {
        return 0;
    }
}
"#;
    let mutations = js_mutations(source, Some("f"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    // if body + else if body + else body = 3
    // (else if is a nested if_statement, so its body and else are also visited)
    assert!(blocks.len() >= 3, "Expected 3 block_remove mutations (if + else if + else), got {}", blocks.len());
}

// --- Loop body removal ---

#[test]
fn for_loop_body_block_remove() {
    let source = r#"
function f(arr) {
    for (let i = 0; i < arr.length; i++) {
        arr[i] = arr[i] + 1;
    }
    return arr;
}
"#;
    let mutations = js_mutations(source, Some("f"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.len() >= 1, "Should create block_remove for for-loop body");
}

#[test]
fn while_loop_body_block_remove() {
    let source = r#"
function f(x) {
    while (x > 0) {
        x = x - 1;
    }
    return x;
}
"#;
    let mutations = js_mutations(source, Some("f"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.len() >= 1, "Should create block_remove for while-loop body");
}

#[test]
fn for_in_loop_body_block_remove() {
    let source = r#"
function f(obj) {
    for (const key in obj) {
        obj[key] = 0;
    }
    return obj;
}
"#;
    let mutations = js_mutations(source, Some("f"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.len() >= 1, "Should create block_remove for for-in loop body");
}

// --- Function scoping ---

#[test]
fn function_scoping_limits_mutations() {
    let source = r#"
function foo(x) {
    return x > 0;
}

function bar(x) {
    return x < 0;
}
"#;
    let all = js_mutations(source, None);
    let foo_only = js_mutations(source, Some("foo"));
    let bar_only = js_mutations(source, Some("bar"));

    assert!(all.len() > foo_only.len());
    assert!(all.len() > bar_only.len());
    assert!(foo_only.iter().all(|m| m.original != "<"));
    assert!(bar_only.iter().all(|m| m.original != ">"));
}

#[test]
fn nonexistent_function_returns_empty() {
    let source = "function foo(x) { return x > 0; }";
    let mutations = js_mutations(source, Some("nonexistent"));
    assert!(mutations.is_empty());
}

// --- list_functions ---

#[test]
fn list_functions_finds_declarations() {
    let source = r#"
function foo() {}
const bar = () => {};
function testSomething() {}
class MyClass {
    baz() {}
    constructor() {}
}
"#;
    let names = parser_js::list_functions(source, JsDialect::JavaScript);
    assert!(names.contains(&"foo".to_string()));
    assert!(names.contains(&"bar".to_string()));
    assert!(names.contains(&"baz".to_string()));
    assert!(!names.contains(&"testSomething".to_string()), "Should skip test functions");
    assert!(!names.contains(&"constructor".to_string()), "Should skip constructor");
}

// --- Skip console.log ---

#[test]
fn skips_console_log() {
    let source = r#"
function f(x) {
    console.log("debug", x);
    return x > 0;
}
"#;
    let mutations = js_mutations(source, Some("f"));
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside console.log");
    }
}

// --- TypeScript specifics ---

#[test]
fn typescript_typed_function() {
    let source = r#"
function check(x: number): boolean {
    return x > 0;
}
"#;
    let mutations = ts_mutations(source, Some("check"));
    assert!(!mutations.is_empty(), "Should parse TypeScript typed functions");
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == ">").collect();
    assert!(cmp.len() >= 2);
}

#[test]
fn typescript_interface_no_mutations() {
    let source = r#"
interface Foo {
    bar: string;
    baz: number;
}

function check(x: number): boolean {
    return x > 0;
}
"#;
    let mutations = ts_mutations(source, Some("check"));
    // Interface should not generate mutations, only the function
    assert!(mutations.iter().all(|m| m.line >= 7));
}

// --- TSX ---

#[test]
fn tsx_function_component() {
    let source = r#"
function Greeting({ name }: { name: string }) {
    if (name === "") {
        return null;
    }
    return <div>Hello {name}</div>;
}
"#;
    let mutations = tsx_mutations(source, Some("Greeting"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == "===").collect();
    assert_eq!(cmp.len(), 1);
    assert_eq!(cmp[0].replacement, "!==");
}

// --- Additional comparison operators ---

#[test]
fn comparison_gte() {
    let source = "function f(x) { return x >= 0; }";
    let mutations = js_mutations(source, Some("f"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == ">=").collect();
    assert_eq!(cmp.len(), 2);
    assert!(cmp.iter().any(|m| m.replacement == ">"));
    assert!(cmp.iter().any(|m| m.replacement == "<"));
}

#[test]
fn comparison_lte() {
    let source = "function f(x) { return x <= 0; }";
    let mutations = js_mutations(source, Some("f"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == "<=").collect();
    assert_eq!(cmp.len(), 2);
    assert!(cmp.iter().any(|m| m.replacement == "<"));
    assert!(cmp.iter().any(|m| m.replacement == ">"));
}

#[test]
fn comparison_lt() {
    let source = "function f(x) { return x < 0; }";
    let mutations = js_mutations(source, Some("f"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == "<").collect();
    assert_eq!(cmp.len(), 2);
    assert!(cmp.iter().any(|m| m.replacement == "<="));
    assert!(cmp.iter().any(|m| m.replacement == ">="));
}

#[test]
fn comparison_ne() {
    let source = "function f(x) { return x != 0; }";
    let mutations = js_mutations(source, Some("f"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == "!=").collect();
    assert_eq!(cmp.len(), 1);
    assert_eq!(cmp[0].replacement, "==");
}

// --- Additional arithmetic operators ---

#[test]
fn arithmetic_minus_to_plus() {
    let source = "function f(a, b) { return a - b; }";
    let mutations = js_mutations(source, Some("f"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith" && m.original == "-").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "+");
}

#[test]
fn arithmetic_multiply() {
    let source = "function f(a, b) { return a * b; }";
    let mutations = js_mutations(source, Some("f"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "/");
}

#[test]
fn arithmetic_divide() {
    let source = "function f(a, b) { return a / b; }";
    let mutations = js_mutations(source, Some("f"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "*");
}

#[test]
fn arithmetic_modulo() {
    let source = "function f(a, b) { return a % b; }";
    let mutations = js_mutations(source, Some("f"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "/");
}

#[test]
fn arithmetic_exponent() {
    let source = "function f(a, b) { return a ** b; }";
    let mutations = js_mutations(source, Some("f"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "*");
}

// --- Binary unknown operator ---

#[test]
fn binary_unknown_operator_no_mutation() {
    let source = "function f(a, b) { return a & b; }";
    let mutations = js_mutations(source, Some("f"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith" || m.operator == "logic_flip" || m.operator == "boundary" || m.operator == "negate_cmp" || m.operator == "negate_eq").collect();
    assert!(arith.is_empty(), "Bitwise & should not produce comparison/logic/arith mutations");
}

// --- Template string concatenation skip ---

#[test]
fn skips_template_string_concatenation() {
    let source = "function f(x) { return `hello` + ` world`; }";
    let mutations = js_mutations(source, Some("f"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 0, "Should not mutate template string concatenation");
}

// --- Additional return mutations ---

#[test]
fn return_bare_becomes_undefined() {
    let source = r#"
function f(x) {
    if (x > 0) {
        return;
    }
    return 1;
}
"#;
    let mutations = js_mutations(source, Some("f"));
    let bare_ret: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val" && m.replacement.contains("undefined")).collect();
    assert_eq!(bare_ret.len(), 1);
}

#[test]
fn return_array_becomes_empty() {
    let source = "function f() { return [1, 2, 3]; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("[]"), "Expected return [];, got: {}", rets[0].replacement);
}

#[test]
fn return_nonempty_object_becomes_empty() {
    let source = "function f() { return {a: 1, b: 2}; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("{}"), "Expected return {{}}, got: {}", rets[0].replacement);
}

#[test]
fn return_undefined_becomes_empty_string() {
    let source = "function f() { return undefined; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("\"\""), "Expected return \"\", got: {}", rets[0].replacement);
}

#[test]
fn return_single_quote_string_becomes_empty() {
    let source = "function f() { return 'hello'; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("\"\""));
}

#[test]
fn return_template_string_becomes_empty() {
    let source = "function f(x) { return `hello ${x}`; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("\"\""));
}

#[test]
fn return_variable_becomes_null() {
    let source = "function f(x) { return x; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("null"), "Expected return null, got: {}", rets[0].replacement);
}

#[test]
fn return_false_becomes_true() {
    let source = "function f() { return false; }";
    let mutations = js_mutations(source, Some("f"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("true"));
}

// --- Boolean false flip ---

#[test]
fn boolean_false_flips() {
    let source = "function f() { let x = false; return x; }";
    let mutations = js_mutations(source, Some("f"));
    let bools: Vec<_> = mutations.iter().filter(|m| m.operator == "bool_flip" && m.original == "false").collect();
    assert_eq!(bools.len(), 1);
    assert_eq!(bools[0].replacement, "true");
}

// --- Additional console.* skips ---

#[test]
fn skips_console_warn() {
    let source = "function f(x) {\n    console.warn(\"w\");\n    return x > 0;\n}";
    let mutations = js_mutations(source, Some("f"));
    for m in &mutations {
        assert!(m.line != 2, "Should not mutate inside console.warn");
    }
}

#[test]
fn skips_console_error() {
    let source = "function f(x) {\n    console.error(\"e\");\n    return x > 0;\n}";
    let mutations = js_mutations(source, Some("f"));
    for m in &mutations {
        assert!(m.line != 2, "Should not mutate inside console.error");
    }
}

#[test]
fn skips_console_info() {
    let source = "function f(x) {\n    console.info(\"i\");\n    return x > 0;\n}";
    let mutations = js_mutations(source, Some("f"));
    for m in &mutations {
        assert!(m.line != 2, "Should not mutate inside console.info");
    }
}

#[test]
fn skips_console_debug() {
    let source = "function f(x) {\n    console.debug(\"d\");\n    return x > 0;\n}";
    let mutations = js_mutations(source, Some("f"));
    for m in &mutations {
        assert!(m.line != 2, "Should not mutate inside console.debug");
    }
}

#[test]
fn skips_use_strict_directive() {
    let source = "function f(x) {\n    'use strict';\n    return x > 0;\n}";
    let mutations = js_mutations(source, Some("f"));
    for m in &mutations {
        assert!(m.operator != "string_mut", "Should not mutate 'use strict' directive");
    }
}

// --- list_functions dialect coverage ---

#[test]
fn list_functions_typescript_dialect() {
    let source = "function check(x: number): boolean { return x > 0; }";
    let names = parser_js::list_functions(source, JsDialect::TypeScript);
    assert!(names.contains(&"check".to_string()));
}

#[test]
fn list_functions_tsx_dialect() {
    let source = "function Greeting({ name }: { name: string }) { return <div>{name}</div>; }";
    let names = parser_js::list_functions(source, JsDialect::Tsx);
    assert!(names.contains(&"Greeting".to_string()));
}

#[test]
fn list_functions_skips_underscore_prefixed() {
    let source = "function _private() {}\nfunction publicFn() {}";
    let names = parser_js::list_functions(source, JsDialect::JavaScript);
    assert!(!names.contains(&"_private".to_string()));
    assert!(names.contains(&"publicFn".to_string()));
}

// --- var declaration arrow function ---

#[test]
fn discovers_var_declaration_arrow() {
    let source = "var check = (x) => { return x > 0; };";
    let mutations = js_mutations(source, Some("check"));
    assert!(!mutations.is_empty(), "Should find mutations in var arrow function");
}

#[test]
fn discovers_exported_arrow_function() {
    let source = "export const check = (x: number): boolean => { return x > 0; };";
    let mutations = ts_mutations(source, Some("check"));
    assert!(!mutations.is_empty(), "Should find mutations in exported arrow function");
}

#[test]
fn collect_all_arrow_functions_no_scope() {
    let source = "const add = (a, b) => { return a + b; };\nconst sub = (a, b) => { return a - b; };";
    let mutations = js_mutations(source, None);
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert!(arith.len() >= 2, "Should find arith mutations from both arrow functions");
}

#[test]
fn list_functions_var_arrow() {
    let source = "var process = (x) => { return x; };";
    let names = parser_js::list_functions(source, JsDialect::JavaScript);
    assert!(names.contains(&"process".to_string()));
}

// --- Empty block not mutated ---

#[test]
fn empty_if_block_not_mutated() {
    let source = "function f(x) {\n    if (x > 0) {}\n    return x;\n}";
    let mutations = js_mutations(source, Some("f"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.is_empty(), "Should not create block_remove for empty {{}} body");
}

// --- Byte offset validity ---

#[test]
fn mutations_have_valid_byte_offsets() {
    let source = r#"
function check(x) {
    return x > 0;
}
"#;
    let mutations = js_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.start_byte < source.len(), "start_byte out of range");
        assert!(m.end_byte <= source.len(), "end_byte out of range");
        assert!(m.start_byte <= m.end_byte, "start_byte > end_byte");
        let original_slice = &source[m.start_byte..m.end_byte];
        assert_eq!(original_slice, m.original, "Byte offsets don't match original text");
    }
}

// --- Empty source ---

#[test]
fn empty_source_returns_no_mutations() {
    let mutations = js_mutations("", None);
    assert!(mutations.is_empty());
}

// --- Context lines ---

#[test]
fn context_lines_are_populated() {
    let source = r#"// line 1
// line 2
function check(x) {
    // line 4
    if (x > 0) {
        return true;
    }
    return false;
}
"#;
    let mutations = js_mutations(source, Some("check"));
    let comparison = mutations.iter().find(|m| m.operator == "boundary").unwrap();
    assert!(!comparison.context_before.is_empty(), "context_before should not be empty");
    assert!(!comparison.context_after.is_empty(), "context_after should not be empty");
}
