use super::*;

#[test]
fn escaped_metacharacter_is_literal() {
    // One generic rule -- "whatever follows `\` is a literal" -- covers every
    // metacharacter uniformly, no per-character special-casing needed.
    assert_eq!(ast("\\*"), lit('*'));
    assert_eq!(ast("\\."), lit('.'));
    assert_eq!(ast("\\("), lit('('));
    assert_eq!(ast("\\["), lit('['));
    assert_eq!(ast("\\\\"), lit('\\'));
    // '^' stops falling through to the literal catch-all once it becomes an
    // anchor -- escaping must still reach a literal caret.
    assert_eq!(ast("\\^"), lit('^'));
}

#[test]
fn escaped_literal_binds_as_a_single_atom_under_quantifiers() {
    // The escaped char is a plain Literal by the time parse_repetition sees
    // it -- a *real*, unescaped operator right after still applies normally.
    assert_eq!(ast("\\*+"), plus(lit('*')));
    assert_eq!(ast("a\\.b"), cat(lit('a'), cat(lit('.'), lit('b'))));
}

#[test]
fn dangling_escape_errors() {
    assert!(parse("\\").is_err());
    assert!(parse("a\\").is_err());
}
