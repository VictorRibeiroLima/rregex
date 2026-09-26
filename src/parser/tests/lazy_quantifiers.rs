use super::*;

// --- lazy quantifiers --------------------------------------------------------
//
// QUANT := ('*' | '+' | '?') '?'? -- the trailing `?` is a modifier on the
// quantifier just consumed, not a second quantifier. This is why
// `stacked_question` (which asserted "a??" == Question(Question(a))) is gone:
// two stacked `?`s and one lazy `?` share the same two characters, and the
// lazy reading wins. Nothing is lost -- Question(Question(a)) was always the
// same language as Question(a), so no string became inexpressible.

#[test]
fn lazy_quantifiers_parse() {
    assert_eq!(ast("a*?"), lazy_star(lit('a')));
    assert_eq!(ast("a+?"), lazy_plus(lit('a')));
    assert_eq!(ast("a??"), lazy_question(lit('a')));
}

#[test]
fn lazy_quantifiers_stack_with_other_operators() {
    // The lazy arms `continue` the loop instead of returning, so whatever
    // comes next still wraps the lazy node like any other repetition.
    assert_eq!(ast("a*?*"), star(lazy_star(lit('a'))));
    assert_eq!(ast("a*+?"), lazy_plus(star(lit('a'))));
}
