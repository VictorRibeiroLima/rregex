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

fn lazy_star(inner: Ast) -> Ast {
    Ast::LazyStar(Box::new(inner))
}

fn lazy_plus(inner: Ast) -> Ast {
    Ast::LazyPlus(Box::new(inner))
}

fn lazy_question(inner: Ast) -> Ast {
    Ast::LazyQuestion(Box::new(inner))
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

#[test]
fn simple_class() {
    assert_eq!(
        ast("[abc]"),
        Ast::Class(
            vec![
                ClassType::Single('a'),
                ClassType::Single('b'),
                ClassType::Single('c')
            ],
            false
        )
    );
}

#[test]
fn simple_range_class() {
    assert_eq!(
        ast("[a-c]"),
        Ast::Class(vec![ClassType::Range('a', 'c')], false)
    );
}

#[test]
fn simple_negated_class() {
    assert_eq!(
        ast("[^abc]"),
        Ast::Class(
            vec![
                ClassType::Single('a'),
                ClassType::Single('b'),
                ClassType::Single('c')
            ],
            true
        )
    );
}

#[test]
fn simple_negated_range_class() {
    assert_eq!(
        ast("[^a-c]"),
        Ast::Class(vec![ClassType::Range('a', 'c')], true)
    );
}

#[test]
fn invalid_range_class() {
    assert!(parse("[c-a]").is_err());
}

#[test]
fn caret_is_literal_when_not_first() {
    // The `start` bug: every branch must reset `start` to false itself, not
    // rely on falling through to the bottom of the loop, or a '^' anywhere
    // in the class (not just position 0) wrongly triggers negation.
    assert_eq!(
        ast("[a^]"),
        Ast::Class(vec![ClassType::Single('a'), ClassType::Single('^')], false)
    );
}

#[test]
fn caret_only_negates_the_whole_class_once() {
    // The first '^' negates and consumes itself; by the second character
    // `start` is already false, so a second '^' is just an ordinary member.
    assert_eq!(
        ast("[^^ab]"),
        Ast::Class(
            vec![
                ClassType::Single('^'),
                ClassType::Single('a'),
                ClassType::Single('b')
            ],
            true
        )
    );
}

#[test]
fn leading_and_trailing_dash_are_literal() {
    assert_eq!(
        ast("[-az]"),
        Ast::Class(
            vec![
                ClassType::Single('-'),
                ClassType::Single('a'),
                ClassType::Single('z')
            ],
            false
        )
    );
    assert_eq!(
        ast("[az-]"),
        Ast::Class(
            vec![
                ClassType::Single('a'),
                ClassType::Single('z'),
                ClassType::Single('-')
            ],
            false
        )
    );
}

#[test]
fn two_leading_dashes_form_a_range() {
    // Not "literal dash, then a dash-to-z range" -- the first '-' is still
    // the pending value when the second '-' is read, so it pairs as the
    // range's start. A '-' is only forced literal when there's no char
    // available on one side to pair with, and here there is one.
    assert_eq!(
        ast("[--z]"),
        Ast::Class(vec![ClassType::Range('-', 'z')], false)
    );
}

#[test]
fn dash_after_a_finished_range_is_literal() {
    // 'd' is fully spent as the end of the first range and can't be reused
    // as the start of a second one -- the second '-' has nothing available
    // before it, so it's read as an ordinary value instead.
    assert_eq!(
        ast("[a-d-z]"),
        Ast::Class(
            vec![
                ClassType::Range('a', 'd'),
                ClassType::Single('-'),
                ClassType::Single('z')
            ],
            false
        )
    );
}

#[test]
fn single_and_range_union_in_one_class() {
    assert_eq!(
        ast("[ab-z]"),
        Ast::Class(
            vec![ClassType::Single('a'), ClassType::Range('b', 'z')],
            false
        )
    );
}

#[test]
fn range_validity_is_checked_per_item_not_across_the_class() {
    // The 'z' at each end is never a candidate for a range check -- only the
    // middle 'a'-'a' pair ever gets compared, independent of what else is in
    // the class.
    assert_eq!(
        ast("[za-az]"),
        Ast::Class(
            vec![
                ClassType::Single('z'),
                ClassType::Range('a', 'a'),
                ClassType::Single('z')
            ],
            false
        )
    );
}

#[test]
fn parens_are_literal_inside_a_class() {
    // No grammar slot for a sub-expression inside `[...]` -- '(' and ')'
    // are just ordinary characters here.
    assert_eq!(
        ast("[(a)]"),
        Ast::Class(
            vec![
                ClassType::Single('('),
                ClassType::Single('a'),
                ClassType::Single(')')
            ],
            false
        )
    );
}

#[test]
fn parens_can_compose_into_an_invalid_range() {
    // ')' followed by '-' followed by '(' reads as the range ')-(', and
    // ')' (0x29) > '(' (0x28) makes it inverted -- not because parens are
    // special-cased, but because the ordinary class grammar happens to
    // compose into an inverted range here.
    assert!(parse("[(ab)-(cd)]").is_err());
}

#[test]
fn mixed_class() {
    assert_eq!(
        ast("[a-c123x-z]"),
        Ast::Class(
            vec![
                ClassType::Range('a', 'c'),
                ClassType::Single('1'),
                ClassType::Single('2'),
                ClassType::Single('3'),
                ClassType::Range('x', 'z')
            ],
            false
        )
    );
}
