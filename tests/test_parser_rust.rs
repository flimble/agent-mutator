use mutator::parser_rust;

#[test]
fn discovers_comparison_mutations() {
    let source = r#"
fn check(x: i32) -> bool {
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.operator == "boundary" || m.operator == "negate_cmp").collect();
    assert!(cmp.len() >= 2, "Expected at least 2 comparison mutations, got {}", cmp.len());
}

#[test]
fn discovers_strict_equality() {
    let source = r#"
fn check(x: i32) -> bool {
    x == 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let eq: Vec<_> = mutations.iter().filter(|m| m.original == "==").collect();
    assert_eq!(eq.len(), 1);
    assert_eq!(eq[0].replacement, "!=");
}

#[test]
fn discovers_not_equal() {
    let source = r#"
fn check(x: i32) -> bool {
    x != 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let neq: Vec<_> = mutations.iter().filter(|m| m.original == "!=").collect();
    assert_eq!(neq.len(), 1);
    assert_eq!(neq[0].replacement, "==");
}

#[test]
fn discovers_logical_and_to_or() {
    let source = r#"
fn check(a: bool, b: bool) -> bool {
    a && b
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let logic: Vec<_> = mutations.iter().filter(|m| m.operator == "logic_flip").collect();
    assert_eq!(logic.len(), 1);
    assert_eq!(logic[0].original, "&&");
    assert_eq!(logic[0].replacement, "||");
}

#[test]
fn discovers_logical_or_to_and() {
    let source = r#"
fn check(a: bool, b: bool) -> bool {
    a || b
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let logic: Vec<_> = mutations.iter().filter(|m| m.operator == "logic_flip").collect();
    assert_eq!(logic.len(), 1);
    assert_eq!(logic[0].replacement, "&&");
}

#[test]
fn discovers_arithmetic_plus_to_minus() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("add"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].original, "+");
    assert_eq!(arith[0].replacement, "-");
}

#[test]
fn discovers_arithmetic_mul_to_div() {
    let source = r#"
fn mul(a: i32, b: i32) -> i32 {
    a * b
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("mul"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "/");
}

#[test]
fn discovers_negation_removal() {
    let source = r#"
fn check(x: bool) -> bool {
    !x
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let nots: Vec<_> = mutations.iter().filter(|m| m.operator == "negate_remove").collect();
    assert_eq!(nots.len(), 1);
}

#[test]
fn discovers_boolean_literal_flip() {
    let source = r#"
fn check() -> bool {
    let x = true;
    x
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let bools: Vec<_> = mutations.iter().filter(|m| m.operator == "bool_flip").collect();
    assert_eq!(bools.len(), 1);
    assert_eq!(bools[0].original, "true");
    assert_eq!(bools[0].replacement, "false");
}

#[test]
fn discovers_return_true_becomes_false() {
    let source = r#"
fn check() -> bool {
    return true;
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("false"));
}

#[test]
fn discovers_return_false_becomes_true() {
    let source = r#"
fn check() -> bool {
    return false;
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("true"));
}

#[test]
fn discovers_return_zero_becomes_one() {
    let source = r#"
fn check() -> i32 {
    return 0;
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("1"));
}

#[test]
fn discovers_if_block_removal() {
    let source = r#"
fn check(x: i32) -> i32 {
    if x > 0 {
        return x + 1;
    }
    return 0;
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.len() >= 1, "Should create block_remove for if body");
    assert_eq!(blocks[0].replacement, "{}");
}

#[test]
fn function_scoping_limits_mutations() {
    let source = r#"
fn foo(x: i32) -> bool {
    x > 0
}

fn bar(x: i32) -> bool {
    x < 0
}
"#;
    let all = parser_rust::discover_mutations(source, None);
    let foo_only = parser_rust::discover_mutations(source, Some("foo"));
    let bar_only = parser_rust::discover_mutations(source, Some("bar"));

    assert!(all.len() > foo_only.len());
    assert!(all.len() > bar_only.len());
    assert!(foo_only.iter().all(|m| m.original != "<"));
    assert!(bar_only.iter().all(|m| m.original != ">"));
}

#[test]
fn nonexistent_function_returns_empty() {
    let source = "fn foo() {}";
    let mutations = parser_rust::discover_mutations(source, Some("nonexistent"));
    assert!(mutations.is_empty());
}

#[test]
fn empty_source_returns_no_mutations() {
    let mutations = parser_rust::discover_mutations("", None);
    assert!(mutations.is_empty());
}

#[test]
fn list_functions_finds_names() {
    let source = r#"
fn foo() {}
fn bar() {}
fn helper() {}
"#;
    let names = parser_rust::list_functions(source);
    assert!(names.contains(&"foo".to_string()));
    assert!(names.contains(&"bar".to_string()));
    assert!(names.contains(&"helper".to_string()));
}

#[test]
fn skips_println_macro() {
    let source = r#"
fn check(x: i32) -> bool {
    println!("debug: {}", x);
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    // Should have comparison mutations but nothing from inside println!
    for m in &mutations {
        assert!(!m.original.contains("debug"), "Should not mutate inside println!");
    }
}

// --- Comparison gte/lte ---

#[test]
fn discovers_gte_mutations() {
    let source = r#"
fn check(x: i32) -> bool {
    x >= 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == ">=").collect();
    assert_eq!(cmp.len(), 2);
    assert!(cmp.iter().any(|m| m.replacement == ">"));
    assert!(cmp.iter().any(|m| m.replacement == "<"));
}

#[test]
fn discovers_lte_mutations() {
    let source = r#"
fn check(x: i32) -> bool {
    x <= 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let cmp: Vec<_> = mutations.iter().filter(|m| m.original == "<=").collect();
    assert_eq!(cmp.len(), 2);
    assert!(cmp.iter().any(|m| m.replacement == "<"));
    assert!(cmp.iter().any(|m| m.replacement == ">"));
}

// --- Arithmetic div/mod ---

#[test]
fn discovers_arithmetic_div_to_mul() {
    let source = r#"
fn div(a: i32, b: i32) -> i32 {
    a / b
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("div"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith" && m.original == "/").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "*");
}

#[test]
fn discovers_arithmetic_mod_to_div() {
    let source = r#"
fn modulo(a: i32, b: i32) -> i32 {
    a % b
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("modulo"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith" && m.original == "%").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "/");
}

#[test]
fn discovers_arithmetic_minus_to_plus() {
    let source = r#"
fn sub(a: i32, b: i32) -> i32 {
    a - b
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("sub"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith" && m.original == "-").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].replacement, "+");
}

// --- Binary unknown operators produce no mutations ---

#[test]
fn bitwise_operators_no_mutations() {
    let source = r#"
fn check(x: i32) -> i32 {
    let a = x << 2;
    let b = x >> 1;
    let c = x & 0xFF;
    let d = x | 0x01;
    a + b + c + d
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    // Only + operators should produce mutations, not << >> & |
    for m in &mutations {
        assert!(m.operator == "arith" && m.original == "+",
            "Only + mutations expected, got {} on '{}'", m.operator, m.original);
    }
}

// --- Unary minus not mutated ---

#[test]
fn unary_minus_not_mutated() {
    let source = r#"
fn negate(x: i32) -> i32 {
    return -x;
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("negate"));
    let nots: Vec<_> = mutations.iter().filter(|m| m.operator == "negate_remove").collect();
    assert!(nots.is_empty(), "Unary - should not produce negate_remove mutation");
}

// --- Skip additional macros ---

#[test]
fn skips_eprintln_macro() {
    let source = r#"
fn check(x: i32) -> bool {
    eprintln!("error: {}", x);
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside eprintln!");
    }
}

#[test]
fn skips_print_macro() {
    let source = r#"
fn check(x: i32) -> bool {
    print!("val: {}", x);
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside print!");
    }
}

#[test]
fn skips_debug_macro() {
    let source = r#"
fn check(x: i32) -> bool {
    debug!("val: {}", x);
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside debug!");
    }
}

#[test]
fn skips_info_macro() {
    let source = r#"
fn check(x: i32) -> bool {
    info!("val: {}", x);
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside info!");
    }
}

#[test]
fn skips_warn_macro() {
    let source = r#"
fn check(x: i32) -> bool {
    warn!("val: {}", x);
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside warn!");
    }
}

#[test]
fn skips_error_macro() {
    let source = r#"
fn check(x: i32) -> bool {
    error!("val: {}", x);
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside error!");
    }
}

#[test]
fn skips_trace_macro() {
    let source = r#"
fn check(x: i32) -> bool {
    trace!("val: {}", x);
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside trace!");
    }
}

// --- Impl block functions ---

#[test]
fn discovers_functions_in_impl_block() {
    let source = r#"
impl Foo {
    fn method_a(x: i32) -> bool { x > 0 }
    fn method_b(x: i32) -> bool { x < 0 }
}
"#;
    let all = parser_rust::discover_mutations(source, None);
    assert!(!all.is_empty(), "Should find mutations inside impl block");
    let a_only = parser_rust::discover_mutations(source, Some("method_a"));
    assert!(!a_only.is_empty(), "Should find method_a inside impl block");
}

// --- Return mutations: various types ---

#[test]
fn return_bare_no_mutation() {
    let source = r#"
fn check() {
    return;
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert!(rets.is_empty(), "Bare return should not produce return_val mutation");
}

#[test]
fn return_unit_no_mutation() {
    let source = r#"
fn check() {
    return ();
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert!(rets.is_empty(), "return () should not produce return_val mutation");
}

#[test]
fn return_string_becomes_empty() {
    let source = r#"
fn check() -> &str {
    return "hello";
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("\"\""), "Expected return \"\".to_string(), got: {}", rets[0].replacement);
}

#[test]
fn return_vec_becomes_empty() {
    let source = r#"
fn check() -> Vec<i32> {
    return vec![1, 2, 3];
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("vec![]"), "Expected return vec![], got: {}", rets[0].replacement);
}

#[test]
fn return_ok_unit_no_mutation() {
    let source = r#"
fn check() -> Result<(), String> {
    return Ok(());
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert!(rets.is_empty(), "return Ok(()) should not produce return_val mutation");
}

#[test]
fn return_number_becomes_default() {
    let source = r#"
fn check() -> i32 {
    return 42;
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert!(rets[0].replacement.contains("Default::default()"), "Expected Default::default(), got: {}", rets[0].replacement);
}

// --- Boolean false literal ---

#[test]
fn boolean_false_flip() {
    let source = r#"
fn check() -> bool {
    let x = false;
    x
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let bools: Vec<_> = mutations.iter().filter(|m| m.operator == "bool_flip" && m.original == "false").collect();
    assert_eq!(bools.len(), 1);
    assert_eq!(bools[0].replacement, "true");
}

// --- Empty if block not mutated ---

#[test]
fn empty_if_block_no_mutation() {
    let source = r#"
fn check(x: i32) {
    if x > 0 {}
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.is_empty(), "Should not create block_remove for empty {{}} body");
}

// --- Context empty after on last line ---

#[test]
fn context_empty_after_last_line() {
    let source = "fn check(x: i32) -> bool {\n    x > 0\n}";
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let cmp = mutations.iter().find(|m| m.operator == "boundary").unwrap();
    assert!(cmp.context_after.is_empty() || cmp.context_after.iter().all(|l| l == "}"),
        "context_after should be empty or just }}, got: {:?}", cmp.context_after);
}

#[test]
fn mutations_have_valid_byte_offsets() {
    let source = r#"
fn check(x: i32) -> bool {
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.start_byte < source.len(), "start_byte out of range");
        assert!(m.end_byte <= source.len(), "end_byte out of range");
        assert!(m.start_byte <= m.end_byte, "start_byte > end_byte");
        let original_slice = &source[m.start_byte..m.end_byte];
        assert_eq!(original_slice, m.original, "Byte offsets don't match original text");
    }
}

#[test]
fn context_lines_are_populated() {
    let source = r#"// line 1
// line 2
fn check(x: i32) -> bool {
    // line 4
    x > 0
}
"#;
    let mutations = parser_rust::discover_mutations(source, Some("check"));
    let comparison = mutations.iter().find(|m| m.operator == "boundary").unwrap();
    assert!(!comparison.context_before.is_empty(), "context_before should not be empty");
}
