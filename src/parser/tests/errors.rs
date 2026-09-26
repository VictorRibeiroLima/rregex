use super::*;

#[test]
fn unclosed_group() {
    assert!(parse("(").is_err());
}

#[test]
fn unmatched_close_paren() {
    // Catches a classic bug: `parse_alternation` stops in front of `)` and
    // returns happily, so the top-level entry point must check that the cursor
    // actually reached the end of input. Without that, this parses as `a`.
    assert!(parse("a)").is_err());
}

#[test]
fn leading_repetition_operator() {
    assert!(parse("*a").is_err());
    assert!(parse("+a").is_err());
    assert!(parse("?a").is_err());
}

#[test]
fn repetition_operator_with_no_atom() {
    assert!(parse("a|*").is_err());
    assert!(parse("a|+").is_err());
    assert!(parse("a|?").is_err());
}
