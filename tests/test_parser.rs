use mutator::parser;

#[test]
fn discovers_comparison_mutations() {
    let source = r#"
def check(x):
    if x > 0:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let comparisons: Vec<_> = mutations.iter().filter(|m| m.operator == "boundary" || m.operator == "negate_cmp").collect();
    assert!(comparisons.len() >= 2, "Expected at least 2 comparison mutations, got {}", comparisons.len());
    assert_eq!(comparisons[0].original, ">");
    assert_eq!(comparisons[0].line, 3);
    assert_eq!(comparisons[0].column, 10);
}

#[test]
fn discovers_boolean_operator_mutations() {
    let source = r#"
def check(a, b):
    if a and b:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let logic: Vec<_> = mutations.iter().filter(|m| m.operator == "logic_flip").collect();
    assert_eq!(logic.len(), 1);
    assert_eq!(logic[0].original, "and");
    assert_eq!(logic[0].replacement, "or");
    assert_eq!(logic[0].line, 3);
    assert_eq!(logic[0].column, 10);
}

#[test]
fn discovers_or_operator_mutations() {
    let source = r#"
def check(a, b):
    if a or b:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let logic: Vec<_> = mutations.iter().filter(|m| m.operator == "logic_flip").collect();
    assert_eq!(logic.len(), 1);
    assert_eq!(logic[0].original, "or");
    assert_eq!(logic[0].replacement, "and");
    assert_eq!(logic[0].line, 3);
}

#[test]
fn skips_docstring_with_mutable_content() {
    let source = r#"
def foo(x):
    """Returns True if x > 0 and x < 10"""
    return x > 0
"#;
    let mutations = parser::discover_mutations(source, Some("foo"));
    // The docstring contains > and < and "and" but they should all be skipped
    for m in &mutations {
        assert!(m.line != 3, "No mutations should come from docstring, got {} at line {}", m.operator, m.line);
    }
    // Only the comparison on line 4 should produce mutations
    let cmps: Vec<_> = mutations.iter().filter(|m| m.operator == "boundary" || m.operator == "negate_cmp").collect();
    assert!(cmps.iter().all(|m| m.line == 4));
}

#[test]
fn discovers_not_operator_mutations() {
    let source = r#"
def check(x):
    if not x:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let nots: Vec<_> = mutations.iter().filter(|m| m.operator == "negate_remove").collect();
    assert_eq!(nots.len(), 1);
    assert_eq!(nots[0].line, 3);
    assert_eq!(nots[0].column, 8);
}

#[test]
fn discovers_return_mutations() {
    let source = r#"
def get_name():
    return "hello"
"#;
    let mutations = parser::discover_mutations(source, Some("get_name"));
    let returns: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(returns.len(), 1);
    assert_eq!(returns[0].replacement, "return \"\"");
    assert_eq!(returns[0].line, 3);
    assert_eq!(returns[0].column, 5);
}

#[test]
fn discovers_arithmetic_mutations() {
    let source = r#"
def add(a, b):
    return a + b
"#;
    let mutations = parser::discover_mutations(source, Some("add"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 1);
    assert_eq!(arith[0].original, "+");
    assert_eq!(arith[0].replacement, "-");
    assert_eq!(arith[0].line, 3);
    assert_eq!(arith[0].column, 14);
}

#[test]
fn discovers_all_arithmetic_operators() {
    let source = r#"
def calc(a, b):
    x = a - b
    y = a * b
    z = a / b
    w = a // b
    m = a % b
    p = a ** b
    return x + y + z + w + m + p
"#;
    let mutations = parser::discover_mutations(source, Some("calc"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    let originals: Vec<&str> = arith.iter().map(|m| m.original.as_str()).collect();
    assert!(originals.contains(&"-"), "Should find - operator");
    assert!(originals.contains(&"*"), "Should find * operator");
    assert!(originals.contains(&"/"), "Should find / operator");
    assert!(originals.contains(&"//"), "Should find // operator");
    assert!(originals.contains(&"%"), "Should find % operator");
    assert!(originals.contains(&"**"), "Should find ** operator");
    assert!(originals.contains(&"+"), "Should find + operator");
}

#[test]
fn discovers_boolean_literal_mutations() {
    let source = r#"
def check():
    x = True
    return x
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let bools: Vec<_> = mutations.iter().filter(|m| m.operator == "bool_flip").collect();
    assert_eq!(bools.len(), 1);
    assert_eq!(bools[0].original, "True");
    assert_eq!(bools[0].replacement, "False");
    assert_eq!(bools[0].line, 3);
    assert_eq!(bools[0].column, 9);
}

#[test]
fn boolean_in_return_not_double_counted() {
    let source = r#"
def check():
    return True
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    // True inside return should be handled by return_val, not bool_flip
    let bools: Vec<_> = mutations.iter().filter(|m| m.operator == "bool_flip").collect();
    assert_eq!(bools.len(), 0, "Bool inside return should not produce bool_flip");
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1, "Should have exactly 1 return_val mutation");
}

#[test]
fn discovers_if_body_removal() {
    let source = r#"
def check(x):
    if x > 0:
        x = x + 1
        return x
    return 0
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.len() >= 1, "Expected at least 1 block_remove mutation");
    assert!(blocks[0].replacement.contains("pass"));
    assert_eq!(blocks[0].column, 9, "block body column should be 9 (indented)");
}

#[test]
fn function_scoping_limits_mutations() {
    let source = r#"
def foo(x):
    return x > 0

def bar(x):
    return x < 0
"#;
    let all = parser::discover_mutations(source, None);
    let foo_only = parser::discover_mutations(source, Some("foo"));
    let bar_only = parser::discover_mutations(source, Some("bar"));

    assert!(all.len() > foo_only.len());
    assert!(all.len() > bar_only.len());
    // foo has > mutations, bar has < mutations — they shouldn't overlap
    assert!(foo_only.iter().all(|m| m.original != "<"));
    assert!(bar_only.iter().all(|m| m.original != ">"));
}

#[test]
fn nonexistent_function_returns_empty() {
    let source = r#"
def foo(x):
    return x > 0
"#;
    let mutations = parser::discover_mutations(source, Some("nonexistent"));
    assert!(mutations.is_empty());
}

#[test]
fn list_functions_finds_names() {
    let source = r#"
def foo():
    pass

def bar():
    pass

def __init__(self):
    pass

def test_something():
    pass
"#;
    let names = parser::list_functions(source);
    assert!(names.contains(&"foo".to_string()));
    assert!(names.contains(&"bar".to_string()));
    // Skips dunder and test_ functions
    assert!(!names.contains(&"__init__".to_string()));
    assert!(!names.contains(&"test_something".to_string()));
}

#[test]
fn skips_print_calls() {
    let source = r#"
def foo():
    print(True and False)
    x = True
    return x
"#;
    let mutations = parser::discover_mutations(source, Some("foo"));
    // print() contains True, False, and `and` but none should be mutated
    for m in &mutations {
        assert!(m.line != 3, "Should not mutate inside print() call, got {} at line {}", m.operator, m.line);
    }
    // But True on line 4 should still be mutated
    let bools: Vec<_> = mutations.iter().filter(|m| m.operator == "bool_flip").collect();
    assert_eq!(bools.len(), 1, "Should have exactly 1 bool_flip (line 4 only)");
    assert_eq!(bools[0].line, 4);
}

#[test]
fn skips_docstrings() {
    let source = r#"
def foo():
    """This is a docstring."""
    return True
"#;
    let mutations = parser::discover_mutations(source, Some("foo"));
    for m in &mutations {
        assert!(m.operator != "string_mut", "Should not mutate docstrings");
        assert!(m.line != 3, "No mutations should come from docstring line");
    }
}

#[test]
fn discovers_is_not_mutations() {
    let source = r#"
def check(x):
    if x is not None:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let is_ops: Vec<_> = mutations.iter().filter(|m| m.original == "is not").collect();
    assert_eq!(is_ops.len(), 1);
    assert_eq!(is_ops[0].replacement, "is");
}

#[test]
fn discovers_is_mutations() {
    let source = r#"
def check(x):
    if x is None:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let is_ops: Vec<_> = mutations.iter().filter(|m| m.original == "is").collect();
    assert_eq!(is_ops.len(), 1);
    assert_eq!(is_ops[0].replacement, "is not");
}

#[test]
fn discovers_in_mutations() {
    let source = r#"
def check(x, items):
    if x in items:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let in_ops: Vec<_> = mutations.iter().filter(|m| m.original == "in").collect();
    assert_eq!(in_ops.len(), 1);
    assert_eq!(in_ops[0].replacement, "not in");
}

#[test]
fn discovers_not_in_mutations() {
    let source = r#"
def check(x, items):
    if x not in items:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let not_in_ops: Vec<_> = mutations.iter().filter(|m| m.original == "not in").collect();
    assert_eq!(not_in_ops.len(), 1);
    assert_eq!(not_in_ops[0].replacement, "in");
}

#[test]
fn skips_logging_calls() {
    let source = r#"
def check(x):
    logging.info(x > 0)
    logging.debug(x > 0)
    logging.warning(x > 0)
    logging.error(x > 0)
    return x > 0
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line >= 7, "Should not mutate inside logging calls, got {} at line {}", m.operator, m.line);
    }
    // Only the comparison on line 7 should produce mutations
    let cmps: Vec<_> = mutations.iter().filter(|m| m.operator == "boundary" || m.operator == "negate_cmp").collect();
    assert_eq!(cmps.len(), 2, "Should have exactly 2 comparison mutations (from line 7 only)");
}

#[test]
fn discovers_bare_return_mutation() {
    let source = r#"
def check():
    return
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert_eq!(rets[0].replacement, "return None");
}

#[test]
fn discovers_return_empty_list() {
    let source = r#"
def check():
    return []
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
}

#[test]
fn skips_string_concat_arithmetic() {
    let source = r#"
def check():
    x = "hello" + " world"
    return x
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let arith: Vec<_> = mutations.iter().filter(|m| m.operator == "arith").collect();
    assert_eq!(arith.len(), 0, "Should not mutate string concatenation");
}

#[test]
fn empty_source_returns_no_mutations() {
    let mutations = parser::discover_mutations("", None);
    assert!(mutations.is_empty());
}

#[test]
fn mutations_have_correct_line_numbers() {
    let source = r#"
def check(x):
    if x > 0:
        return True
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    for m in &mutations {
        assert!(m.line >= 2, "Line should be >= 2 (inside function), got {}", m.line);
        assert!(m.line <= 5, "Line should be <= 5, got {}", m.line);
    }
}

#[test]
fn mutations_have_valid_byte_offsets() {
    let source = r#"
def check(x):
    return x > 0
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
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
    let source = r#"# line 1
# line 2
def check(x):
    # line 4
    if x > 0:
        return True
    # line 7
    return False
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let comparison = mutations.iter().find(|m| m.operator == "boundary").unwrap();
    assert!(!comparison.context_before.is_empty(), "context_before should not be empty");
    assert!(!comparison.context_after.is_empty(), "context_after should not be empty");
    // context_after should include the line after the mutation
    assert!(comparison.context_after[0].contains("return True"),
        "First context_after line should contain 'return True', got: {:?}", comparison.context_after);
}

#[test]
fn context_after_does_not_include_current_line() {
    let source = r#"
def check(x, y):
    a = x
    b = x > y
    c = y
    return b
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let cmp = mutations.iter().find(|m| m.operator == "boundary").unwrap();
    // The mutation is on `>` at line 4. context_after should start with line 5, not line 4.
    assert!(!cmp.context_after.is_empty());
    assert!(cmp.context_after[0].contains("c = y"),
        "First context_after should be the line AFTER the mutation, got: {:?}", cmp.context_after);
    // context_before should include lines before
    assert!(!cmp.context_before.is_empty());
    assert!(cmp.context_before.last().unwrap().contains("a = x"),
        "Last context_before should be line before mutation, got: {:?}", cmp.context_before);
}

#[test]
fn context_at_end_of_function() {
    let source = "def check(x):\n    return x > 0\n";
    let mutations = parser::discover_mutations(source, Some("check"));
    let cmp = mutations.iter().find(|m| m.operator == "boundary").unwrap();
    assert!(cmp.line == 2);
}

#[test]
fn context_on_first_line() {
    let source = "def f():\n    x = 1 > 0\n    y = 2\n    z = 3\n";
    let mutations = parser::discover_mutations(source, Some("f"));
    let cmp = mutations.iter().find(|m| m.operator == "boundary").unwrap();
    assert_eq!(cmp.line, 2);
    // context_before should have "def f():" only
    assert_eq!(cmp.context_before.len(), 1);
    assert!(cmp.context_before[0].contains("def f()"));
    // context_after should have "y = 2" and "z = 3"
    assert_eq!(cmp.context_after.len(), 2);
    assert!(cmp.context_after[0].contains("y = 2"));
    assert!(cmp.context_after[1].contains("z = 3"));
}

#[test]
fn bare_return_has_correct_line_and_column() {
    let source = "def check():\n    return\n";
    let mutations = parser::discover_mutations(source, Some("check"));
    let rets: Vec<_> = mutations.iter().filter(|m| m.operator == "return_val").collect();
    assert_eq!(rets.len(), 1);
    assert_eq!(rets[0].line, 2);
    assert_eq!(rets[0].column, 5);
}

#[test]
fn if_body_mutation_has_correct_line() {
    let source = r#"
def check(x):
    if x > 0:
        x = x + 1
        return x
    return 0
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.len() >= 1);
    assert_eq!(blocks[0].line, 4, "block_remove should point to the first line of the block body");
}

#[test]
fn if_pass_body_not_mutated() {
    let source = r#"
def check(x):
    if x > 0:
        pass
    return x
"#;
    let mutations = parser::discover_mutations(source, Some("check"));
    let blocks: Vec<_> = mutations.iter().filter(|m| m.operator == "block_remove").collect();
    assert!(blocks.is_empty(), "pass body should not generate block_remove");
}
