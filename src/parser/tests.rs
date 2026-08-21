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

fn cat(children: Vec<Ast>) -> Ast {
    Ast::Concat(children)
}

fn alt(branches: Vec<Ast>) -> Ast {
    Ast::Alternation(branches)
}

fn star(inner: Ast) -> Ast {
    Ast::Star(Box::new(inner))
}

// --- should parse ----------------------------------------------------------

#[test]
fn literal() {
    assert_eq!(ast("a"), lit('a'));
}

#[test]
fn concat() {
    assert_eq!(ast("ab"), cat(vec![lit('a'), lit('b')]));
}

#[test]
fn alternation() {
    assert_eq!(ast("a|b"), alt(vec![lit('a'), lit('b')]));
}

#[test]
fn concat_binds_tighter_than_alternation() {
    // `(ab)|(cd)`, never `a(b|c)d` — the root must be the Alternation.
    assert_eq!(
        ast("ab|cd"),
        alt(vec![
            cat(vec![lit('a'), lit('b')]),
            cat(vec![lit('c'), lit('d')]),
        ])
    );
}

#[test]
fn star_binds_to_the_single_preceding_atom() {
    // `a|(b(c*))` — the star takes `c` only, not `bc`.
    assert_eq!(
        ast("a|bc*"),
        alt(vec![lit('a'), cat(vec![lit('b'), star(lit('c'))])])
    );
}

#[test]
fn group_makes_an_expression_into_one_atom() {
    // The parens force the shape, then vanish: nothing records they were written.
    assert_eq!(
        ast("(a|b)*c"),
        cat(vec![star(alt(vec![lit('a'), lit('b')])), lit('c')])
    );
}

#[test]
fn stacked_stars() {
    // `a**` is `(a*)*`. It must PARSE — it compiles to an NFA with an
    // epsilon-loop, a cycle consuming no input, which is the matcher's problem.
    assert_eq!(ast("a**"), star(star(lit('a'))));
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
}

#[test]
fn repetition_operator_with_no_atom() {
    assert!(parse("a|*").is_err());
}

// --- empty branches --------------------------------------------------------
//
// The permissive (PCRE-style) choice: an empty branch is legal and produces
// `Ast::Empty`, so `a|` matches "a" OR "" and is equivalent to `a?`.
//
// Three tests because the empty branch reaches `parse_alternation` by three
// different routes: at EOF, at the very start of the loop, and between two
// consecutive `|`. A parser can get one right and the others wrong.

#[test]
fn trailing_empty_branch() {
    assert_eq!(ast("a|"), alt(vec![lit('a'), Ast::Empty]));
}

#[test]
fn leading_empty_branch() {
    assert_eq!(ast("|a"), alt(vec![Ast::Empty, lit('a')]));
}

#[test]
fn empty_branch_between_two_alternatives() {
    assert_eq!(ast("a||b"), alt(vec![lit('a'), Ast::Empty, lit('b')]));
}
