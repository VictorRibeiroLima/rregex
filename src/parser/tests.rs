use super::*;

/// Parse, or fail the test with a readable message.
/// Written out instead of `.unwrap()` so `ParserError` doesn't need `Debug`.
fn ast(input: &str) -> Ast {
    match parse(input) {
        Ok(a) => a,
        Err(_) => panic!("expected `{input}` to parse, got an error"),
    }
}

fn lit(c: char) -> Ast {
    Ast::Literal(c)
}

fn cat(left: Ast, right: Ast) -> Ast {
    Ast::Concat(Box::new(left), Box::new(right))
}

fn alt(left: Ast, right: Ast) -> Ast {
    Ast::Alternation(Box::new(left), Box::new(right))
}

fn star(inner: Ast) -> Ast {
    Ast::Star(Box::new(inner))
}

fn plus(inner: Ast) -> Ast {
    Ast::Plus(Box::new(inner))
}

fn question(inner: Ast) -> Ast {
    Ast::Question(Box::new(inner))
}

// --- should parse ----------------------------------------------------------

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
    assert_eq!(ast("a|bc?"), alt(lit('a'), cat(lit('b'), question(lit('c')))));
}

#[test]
fn stacked_plus() {
    assert_eq!(ast("a++"), plus(plus(lit('a'))));
}

#[test]
fn stacked_question() {
    assert_eq!(ast("a??"), question(question(lit('a'))));
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

// --- should error ----------------------------------------------------------

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
