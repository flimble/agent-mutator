use mutator::operators;

// --- String mutations ---

#[test]
fn string_empty_becomes_mutator_xx() {
    let ops = operators::string_mutations("\"\"");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "\"mutator_xx\"");
}

#[test]
fn string_single_quote_empty_becomes_mutator_xx() {
    let ops = operators::string_mutations("''");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "\"mutator_xx\"");
}

#[test]
fn string_nonempty_becomes_empty() {
    let ops = operators::string_mutations("\"hello\"");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "\"\"");
}

// --- Conditional body removal ---

#[test]
fn conditional_body_removal_returns_pass() {
    let ops = operators::conditional_body_removal();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].operator_name, "block_remove");
    assert_eq!(ops[0].replacement, "pass");
}

#[test]
fn comparison_gt_produces_boundary_and_negate() {
    let ops = operators::comparison_mutations(">");
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].replacement, ">=");
    assert_eq!(ops[0].operator_name, "boundary");
    assert_eq!(ops[1].replacement, "<=");
    assert_eq!(ops[1].operator_name, "negate_cmp");
}

#[test]
fn comparison_gte_produces_boundary_and_negate() {
    let ops = operators::comparison_mutations(">=");
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].replacement, ">");
    assert_eq!(ops[1].replacement, "<");
}

#[test]
fn comparison_lt_produces_boundary_and_negate() {
    let ops = operators::comparison_mutations("<");
    assert_eq!(ops[0].replacement, "<=");
    assert_eq!(ops[1].replacement, ">=");
}

#[test]
fn comparison_lte_produces_boundary_and_negate() {
    let ops = operators::comparison_mutations("<=");
    assert_eq!(ops[0].replacement, "<");
    assert_eq!(ops[1].replacement, ">");
}

#[test]
fn comparison_eq_produces_negate() {
    let ops = operators::comparison_mutations("==");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "!=");
}

#[test]
fn comparison_neq_produces_negate() {
    let ops = operators::comparison_mutations("!=");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "==");
}

#[test]
fn comparison_is_produces_is_not() {
    let ops = operators::comparison_mutations("is");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "is not");
}

#[test]
fn comparison_is_not_produces_is() {
    let ops = operators::comparison_mutations("is not");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "is");
}

#[test]
fn comparison_in_produces_not_in() {
    let ops = operators::comparison_mutations("in");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "not in");
}

#[test]
fn comparison_not_in_produces_in() {
    let ops = operators::comparison_mutations("not in");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "in");
}

#[test]
fn comparison_unknown_returns_empty() {
    let ops = operators::comparison_mutations("??");
    assert!(ops.is_empty());
}

#[test]
fn boolean_true_flips_to_false() {
    let ops = operators::boolean_mutations("True");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "False");
}

#[test]
fn boolean_false_flips_to_true() {
    let ops = operators::boolean_mutations("False");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "True");
}

#[test]
fn boolean_unknown_returns_empty() {
    assert!(operators::boolean_mutations("maybe").is_empty());
}

#[test]
fn logical_and_flips_to_or() {
    let ops = operators::logical_mutations("and");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "or");
}

#[test]
fn logical_or_flips_to_and() {
    let ops = operators::logical_mutations("or");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "and");
}

#[test]
fn logical_not_removes() {
    let ops = operators::logical_mutations("not");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "");
}

#[test]
fn logical_unknown_returns_empty() {
    assert!(operators::logical_mutations("xor").is_empty());
}

#[test]
fn return_none_becomes_empty_string() {
    let ops = operators::return_mutations("None");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].replacement, "return \"\"");
}

#[test]
fn return_true_becomes_false() {
    let ops = operators::return_mutations("True");
    assert_eq!(ops[0].replacement, "return False");
}

#[test]
fn return_false_becomes_true() {
    let ops = operators::return_mutations("False");
    assert_eq!(ops[0].replacement, "return True");
}

#[test]
fn return_string_becomes_empty() {
    let ops = operators::return_mutations("\"hello\"");
    assert_eq!(ops[0].replacement, "return \"\"");
}

#[test]
fn return_fstring_becomes_empty() {
    let ops = operators::return_mutations("f\"hello {x}\"");
    assert_eq!(ops[0].replacement, "return \"\"");
}

#[test]
fn return_list_becomes_empty_list() {
    let ops = operators::return_mutations("[1, 2, 3]");
    assert_eq!(ops[0].replacement, "return []");
}

#[test]
fn return_dict_becomes_empty_dict() {
    let ops = operators::return_mutations("{\"a\": 1}");
    assert_eq!(ops[0].replacement, "return {}");
}

#[test]
fn return_zero_becomes_one() {
    let ops = operators::return_mutations("0");
    assert_eq!(ops[0].replacement, "return 1");
}

#[test]
fn return_number_becomes_zero() {
    let ops = operators::return_mutations("42");
    assert_eq!(ops[0].replacement, "return 0");
}

#[test]
fn return_expression_becomes_none() {
    let ops = operators::return_mutations("some_func()");
    assert_eq!(ops[0].replacement, "return None");
}

#[test]
fn arithmetic_plus_to_minus() {
    let ops = operators::arithmetic_mutations("+");
    assert_eq!(ops[0].replacement, "-");
}

#[test]
fn arithmetic_minus_to_plus() {
    let ops = operators::arithmetic_mutations("-");
    assert_eq!(ops[0].replacement, "+");
}

#[test]
fn arithmetic_mul_to_div() {
    let ops = operators::arithmetic_mutations("*");
    assert_eq!(ops[0].replacement, "/");
}

#[test]
fn arithmetic_div_to_mul() {
    let ops = operators::arithmetic_mutations("/");
    assert_eq!(ops[0].replacement, "*");
}

#[test]
fn arithmetic_floordiv_to_div() {
    let ops = operators::arithmetic_mutations("//");
    assert_eq!(ops[0].replacement, "/");
}

#[test]
fn arithmetic_mod_to_div() {
    let ops = operators::arithmetic_mutations("%");
    assert_eq!(ops[0].replacement, "/");
}

#[test]
fn arithmetic_pow_to_mul() {
    let ops = operators::arithmetic_mutations("**");
    assert_eq!(ops[0].replacement, "*");
}

#[test]
fn arithmetic_unknown_returns_empty() {
    assert!(operators::arithmetic_mutations("^").is_empty());
}
