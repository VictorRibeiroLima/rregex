use super::*;

// --- empty branches --------------------------------------------------------
//
// The permissive (PCRE-style) choice: an empty branch is legal and produces
// `Ast::Empty`, so `a|` matches "a" OR "" and is equivalent to `a?`.
//
// Three tests because the empty branch reaches `parse_alternation` by three
// different routes: at EOF, at the very start of the loop, and between two
// consecutive `|`. A parser can get one right and the others wrong.
//
// Now that the node is binary and right-associative, `a||b` nests: the second
// branch of the outer Alternation is itself an Alternation.

#[test]
fn trailing_empty_branch() {
    assert_eq!(ast("a|"), alt(lit('a'), Ast::Empty));
}

#[test]
fn leading_empty_branch() {
    assert_eq!(ast("|a"), alt(Ast::Empty, lit('a')));
}

#[test]
fn empty_branch_between_two_alternatives() {
    assert_eq!(ast("a||b"), alt(lit('a'), alt(Ast::Empty, lit('b'))));
}
