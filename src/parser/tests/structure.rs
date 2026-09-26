use super::*;

#[test]
fn literal() {
    assert_eq!(ast("a"), lit('a'));
}

#[test]
fn concat() {
    assert_eq!(ast("ab"), cat(lit('a'), lit('b')));
}

#[test]
fn alternation() {
    assert_eq!(ast("a|b"), alt(lit('a'), lit('b')));
}

#[test]
fn concat_binds_tighter_than_alternation() {
    // `(ab)|(cd)`, never `a(b|c)d` — the root must be the Alternation.
    assert_eq!(
        ast("ab|cd"),
        alt(cat(lit('a'), lit('b')), cat(lit('c'), lit('d')))
    );
}

#[test]
fn star_binds_to_the_single_preceding_atom() {
    // `a|(b(c*))` — the star takes `c` only, not `bc`.
    assert_eq!(ast("a|bc*"), alt(lit('a'), cat(lit('b'), star(lit('c')))));
}

#[test]
fn plus_binds_to_the_single_preceding_atom() {
    // `a|(b(c+))` -- same argument as the `*` case: the operator takes `c`
    // only, not `bc`.
    assert_eq!(ast("a|bc+"), alt(lit('a'), cat(lit('b'), plus(lit('c')))));
}

#[test]
fn question_binds_to_the_single_preceding_atom() {
    assert_eq!(
        ast("a|bc?"),
        alt(lit('a'), cat(lit('b'), question(lit('c'))))
    );
}

#[test]
fn stacked_plus() {
    assert_eq!(ast("a++"), plus(plus(lit('a'))));
}

#[test]
fn repetition_operators_stack_left_to_right_when_mixed() {
    // parse_repetition is a loop, not a single dispatch: each operator wraps
    // whatever the previous one built, so different operators compose the
    // same way repeated `*` already does.
    assert_eq!(ast("a+*"), star(plus(lit('a'))));
    assert_eq!(ast("a?*"), star(question(lit('a'))));
    assert_eq!(ast("a*+"), plus(star(lit('a'))));
}

#[test]
fn group_makes_an_expression_into_one_atom() {
    // The parens force the shape, then vanish: nothing records they were written.
    assert_eq!(ast("(a|b)*c"), cat(star(alt(lit('a'), lit('b'))), lit('c')));
}

#[test]
fn dot_parses_to_any() {
    assert_eq!(ast("."), Ast::Any);
}

#[test]
fn dot_binds_as_a_single_atom() {
    // `.` sits at atom level, same as a Literal -- it must take part in
    // Concat like any other atom, not swallow or get swallowed by a neighbor.
    assert_eq!(ast("a.b"), cat(lit('a'), cat(Ast::Any, lit('b'))));
}

#[test]
fn dot_can_be_starred() {
    assert_eq!(ast(".*"), star(Ast::Any));
}

#[test]
fn stacked_stars() {
    // `a**` is `(a*)*`. It must PARSE — it compiles to an NFA with an
    // epsilon-loop, a cycle consuming no input, which is the matcher's problem.
    assert_eq!(ast("a**"), star(star(lit('a'))));
}

#[test]
fn concat_is_right_associative() {
    // Nothing in the language cares — concatenation is associative, so
    // `Concat(a, Concat(b, c))` and `Concat(Concat(a, b), c)` describe the same
    // set of strings. But the binary node forces a choice, and the choice is
    // now visible in the tree, so pin it down before the compiler starts
    // depending on it.
    assert_eq!(ast("abc"), cat(lit('a'), cat(lit('b'), lit('c'))));
}

#[test]
fn alternation_is_right_associative() {
    // Same argument as `abc`: `|` is associative, the node shape is not.
    assert_eq!(ast("a|b|c"), alt(lit('a'), alt(lit('b'), lit('c'))));
}
