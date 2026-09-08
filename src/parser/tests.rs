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

fn br(inner: Ast, n: u16, m: Option<u16>, lazy: bool) -> Ast {
    Ast::BoundedRepetition(BoundedRepetition {
        ast: Box::new(inner),
        n,
        m,
        lazy,
    })
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
            ClassSet::from_vec(vec![
                ClassType::Single('a'),
                ClassType::Single('b'),
                ClassType::Single('c')
            ],),
            false
        )
    );
}

#[test]
fn simple_range_class() {
    assert_eq!(
        ast("[a-c]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Range('a', 'c')]), false)
    );
}

#[test]
fn simple_negated_class() {
    assert_eq!(
        ast("[^abc]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Single('a'),
                ClassType::Single('b'),
                ClassType::Single('c')
            ]),
            true
        )
    );
}

#[test]
fn simple_negated_range_class() {
    assert_eq!(
        ast("[^a-c]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Range('a', 'c')]), true)
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
        Ast::Class(
            ClassSet::from_vec(vec![ClassType::Single('a'), ClassType::Single('^')]),
            false
        )
    );
}

#[test]
fn caret_only_negates_the_whole_class_once() {
    // The first '^' negates and consumes itself; by the second character
    // `start` is already false, so a second '^' is just an ordinary member.
    assert_eq!(
        ast("[^^ab]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Single('^'),
                ClassType::Single('a'),
                ClassType::Single('b')
            ]),
            true
        )
    );
}

#[test]
fn leading_and_trailing_dash_are_literal() {
    assert_eq!(
        ast("[-az]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Single('-'),
                ClassType::Single('a'),
                ClassType::Single('z')
            ]),
            false
        )
    );
    assert_eq!(
        ast("[az-]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Single('a'),
                ClassType::Single('z'),
                ClassType::Single('-')
            ]),
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
        Ast::Class(ClassSet::from_vec(vec![ClassType::Range('-', 'z')]), false)
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
            ClassSet::from_vec(vec![
                ClassType::Range('a', 'd'),
                ClassType::Single('-'),
                ClassType::Single('z')
            ]),
            false
        )
    );
}

#[test]
fn single_and_range_union_in_one_class() {
    assert_eq!(
        ast("[ab-z]"),
        Ast::Class(
            ClassSet::from_vec(vec![ClassType::Single('a'), ClassType::Range('b', 'z')]),
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
            ClassSet::from_vec(vec![ClassType::Single('z'), ClassType::Range('a', 'a')]), //Only one single('z') because this is a set
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
            ClassSet::from_vec(vec![
                ClassType::Single('('),
                ClassType::Single('a'),
                ClassType::Single(')')
            ]),
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

#[test]
fn mixed_class() {
    assert_eq!(
        ast("[a-c123x-z]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Range('a', 'c'),
                ClassType::Single('1'),
                ClassType::Single('2'),
                ClassType::Single('3'),
                ClassType::Range('x', 'z')
            ]),
            false
        )
    );
}

// --- empty class ------------------------------------------------------------
//
// `parse_class`'s loop checks for ']' before pushing anything, so a class
// that closes with zero items parses -- permissive, same family as the empty
// branch. `[]` is not sugar for an error: it is the one place in the grammar
// that gets a spelling for the empty language (Kleene's ∅), as opposed to
// `Ast::Empty` which is the empty *string* (ε). `Class`'s union-scan-then-
// negate-once representation already produces the right value for zero items
// with no special case, so `[^]` -- negate an empty union -- comes out as
// "matches every character," the same language as `.` by a different route.

#[test]
fn empty_class_parses_to_a_class_with_no_members() {
    assert_eq!(ast("[]"), Ast::Class(ClassSet::from_vec(vec![]), false));
}

#[test]
fn negated_empty_class_parses_to_a_negated_class_with_no_members() {
    assert_eq!(ast("[^]"), Ast::Class(ClassSet::from_vec(vec![]), true));
}

// --- class escapes (deliberately red) ---------------------------------------
//
// parse_class has no `\` handling at all -- unlike parse_atom's `\` branch,
// which it doesn't call into, every char inside `[...]` (backslash included)
// is read as itself. These pin the behavior a real escape rule should give --
// "whatever follows `\` is a literal", the same rule parse_atom already
// applies. Red until Lesson 6, which opens by fixing this: escapes inside
// `[...]` are a prerequisite for `\d`/`\w`/`\s` riding on the same branch.

#[test]
fn escaping_the_closing_bracket_is_currently_unsupported() {
    // Intended: `[\]]` is a one-member class holding a literal `]` -- the
    // only way to put `]` in a class at all, since an unescaped `]` always
    // closes the class (there's no "`]` right after `[` is literal"
    // convention here). Actual: the `\` is read as an ordinary char, so the
    // very next `]` still closes the class -- the escape is invisible to it.
    // The leftover `]` isn't a stop character outside a class (only `|`, `)`,
    // EOF are), so it doesn't even error: it falls through to parse_atom's
    // `Literal(c)` catch-all, and the whole thing silently parses as
    // `Concat(Class({\\}), Literal(']'))`.
    assert_eq!(
        ast("[\\]]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Single(']')]), false)
    );
}

#[test]
fn escaping_a_class_metacharacter_should_not_leave_a_stray_backslash_member() {
    // Intended: `\-` and `\^` are each a single literal member. Actual: `\` is
    // pushed as its own ordinary Single('\\'), and the following char is
    // pushed separately -- two members instead of one, with a backslash
    // neither pattern asked for.
    assert_eq!(
        ast("[\\-]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Single('-')]), false)
    );
    assert_eq!(
        ast("[\\^]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Single('^')]), false)
    );
}

#[test]
fn an_escape_must_not_be_eaten_as_a_range_endpoint() {
    // Intended: `[a-\]]` is the range 'a'..']' -- and the escape isn't
    // optional there, since an unescaped `]` would close the class instead.
    // Actual: the range logic sees `-` followed by a char and takes that char
    // as the endpoint, but the char it finds is the backslash, so it builds
    // 'a'..'\\' -- inverted, because '\\' (0x5C) < 'a' (0x61) -- and reports
    // InvalidRange for a range the pattern never wrote.
    //
    // The order this implies: an escape has to resolve to its char *before*
    // range detection runs, not after. Note that resolving it isn't enough on
    // its own -- once `\d` exists, an escape can resolve to a whole class,
    // which is not a legal endpoint at all.
    assert_eq!(
        ast("[a-\\]]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Range('a', ']')]), false)
    );
}

#[test]
fn an_escaped_closing_bracket_does_not_terminate_a_class() {
    // The mirror image of the two above: here the parser wrongly *accepts*.
    // `[abc\]` escapes its only `]`, so the class is never closed and this is
    // an unterminated class. Actual: the `\` is pushed as an ordinary member
    // and the `]` right behind it still reads as the terminator, so the whole
    // thing parses happily as the class {a,b,c,\\} -- the silent-success
    // failure mode, same family as `a)` before the toplevel EOF check existed.
    assert!(parse("[abc\\]").is_err());
}

//-----------Bounded Repetition Tests-----------------

#[test]
fn bounded_repetition_at_most_m() {
    assert_eq!(ast("a{,3}"), br(lit('a'), 0, Some(3), false));
    assert_eq!(ast("a{,33333}"), br(lit('a'), 0, Some(33333), false));
    assert_eq!(
        ast("a{                 ,              3             }"),
        br(lit('a'), 0, Some(3), false)
    );
    assert_eq!(
        ast("a{                 ,              3             }?"),
        br(lit('a'), 0, Some(3), true)
    );
    assert_eq!(
        ast("a{                 ,              3             }?b"),
        cat(br(lit('a'), 0, Some(3), true), lit('b'))
    );
    assert_eq!(
        ast("a{                 ,              3             } ?b"),
        cat(
            br(lit('a'), 0, Some(3), false),
            cat(question(lit(' ')), lit('b'))
        )
    );
}

#[test]
fn bounded_repetition_fake_at_most_m() {
    assert_eq!(
        ast("a{,a3}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(lit(','), cat(lit('a'), cat(lit('3'), lit('}'))))
            )
        )
    );

    assert_eq!(
        ast("a{,3a}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(lit(','), cat(lit('3'), cat(lit('a'), lit('}'))))
            )
        )
    );
}

#[test]
fn bounded_repetition_at_most_m_with_overflow() {
    let result = parse("a{,65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{,65536}".to_string()));

    let result = parse("a{          ,              65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          ,              65536}".to_string())
    );
}

#[test]
fn bounded_repetition_exactly_n() {
    assert_eq!(ast("a{3}"), br(lit('a'), 3, Some(3), false));
    assert_eq!(ast("a{33333}"), br(lit('a'), 33333, Some(33333), false));
    assert_eq!(
        ast("a{                 3             }"),
        br(lit('a'), 3, Some(3), false)
    );
    assert_eq!(
        ast("a{                 3             }?"),
        br(lit('a'), 3, Some(3), true)
    );
    assert_eq!(
        ast("a{                 3             }?b"),
        cat(br(lit('a'), 3, Some(3), true), lit('b'))
    );
    assert_eq!(
        ast("a{                 3             } ?b"),
        cat(
            br(lit('a'), 3, Some(3), false),
            cat(question(lit(' ')), lit('b'))
        )
    );
}

#[test]
fn bounded_repetition_exactly_n_with_overflow() {
    let result = parse("a{65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{65536}".to_string()));
    let result = parse("a{          65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          65536}".to_string())
    );
}

#[test]
fn bounded_repetition_fake_exactly_n() {
    assert_eq!(
        ast("a{3a}"),
        cat(
            lit('a'),
            cat(lit('{'), cat(lit('3'), cat(lit('a'), lit('}'))))
        )
    );
}

#[test]
fn bounded_repetition_at_least_n() {
    assert_eq!(ast("a{3,}"), br(lit('a'), 3, None, false));
    assert_eq!(ast("a{33333,}"), br(lit('a'), 33333, None, false));
    assert_eq!(
        ast("a{                 3             ,}"),
        br(lit('a'), 3, None, false)
    );
    assert_eq!(
        ast("a{                 3             ,}?"),
        br(lit('a'), 3, None, true)
    );
    assert_eq!(
        ast("a{                 3             ,}?b"),
        cat(br(lit('a'), 3, None, true), lit('b'))
    );
    assert_eq!(
        ast("a{                 3             ,} ?b"),
        cat(
            br(lit('a'), 3, None, false),
            cat(question(lit(' ')), lit('b'))
        )
    );
}

#[test]
fn bounded_repetition_at_least_n_with_overflow() {
    let result = parse("a{65536,}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{65536,}".to_string()));
    let result = parse("a{          65536,}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          65536,}".to_string())
    );
    let result = parse("a{          65536,     }");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          65536,     }".to_string())
    );
}

#[test]
fn bounded_repetition_fake_at_least_n() {
    assert_eq!(
        ast("a{3,a}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(lit('3'), cat(lit(','), cat(lit('a'), lit('}'))))
            )
        )
    );
}

#[test]
fn bounded_repetition_n_m() {
    assert_eq!(ast("a{3,5}"), br(lit('a'), 3, Some(5), false));
    assert_eq!(
        ast("a{33333,55555}"),
        br(lit('a'), 33333, Some(55555), false)
    );
    assert_eq!(
        ast("a{                 3             ,              5             }"),
        br(lit('a'), 3, Some(5), false)
    );
    assert_eq!(
        ast("a{                 3             ,              5             }?"),
        br(lit('a'), 3, Some(5), true)
    );
    assert_eq!(
        ast("a{                 3             ,              5             }?b"),
        cat(br(lit('a'), 3, Some(5), true), lit('b'))
    );
    assert_eq!(
        ast("a{                 3             ,              5             } ?b"),
        cat(
            br(lit('a'), 3, Some(5), false),
            cat(question(lit(' ')), lit('b'))
        )
    );
}

#[test]
fn bounded_repetition_n_greater_than_m() {
    let result = parse("a{3,2}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::MinimumGreaterThanMaximum(3, 2));
}

#[test]
fn bounded_repetition_n_greater_than_m_with_overflow() {
    let result = parse("a{65536,65535}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{65536,65535}".to_string()));
    let result = parse("a{65535,65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{65535,65536}".to_string()));
    let result = parse("a{          65536,     65535}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          65536,     65535}".to_string())
    );
}

#[test]
fn bounded_repetition_fake_n_greater_than_m() {
    assert_eq!(
        ast("a{3,2a}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(
                    lit('3'),
                    cat(lit(','), cat(lit('2'), cat(lit('a'), lit('}'))))
                )
            )
        )
    );
}

#[test]
fn bounded_repetition_fake_n_greater_than_m_with_overflow() {
    assert_eq!(
        ast("a{65536,65535a}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(
                    lit('6'),
                    cat(
                        lit('5'),
                        cat(
                            lit('5'),
                            cat(
                                lit('3'),
                                cat(
                                    lit('6'),
                                    cat(
                                        lit(','),
                                        cat(
                                            lit('6'),
                                            cat(
                                                lit('5'),
                                                cat(
                                                    lit('5'),
                                                    cat(
                                                        lit('3'),
                                                        cat(lit('5'), cat(lit('a'), lit('}')))
                                                    )
                                                )
                                            )
                                        )
                                    )
                                )
                            )
                        )
                    )
                )
            )
        )
    );
}

// --- stacking with other quantifiers ----------------------------------------
//
// `{n,m}` is a quantifier like any other, so `parse_repetition`'s loop must
// let a following `*`/`+`/`{n,m}` wrap it, the same way it already lets `*`
// wrap `+` in `a+*`. Stacking is always language-redundant (same argument as
// `stacked_plus`/`repetition_operators_stack_left_to_right_when_mixed`) but
// syntactically legal -- `{n,m}` isn't special enough to be the one
// quantifier that can't be stacked onto.
//
// A trailing `?` is deliberately excluded from this section: it's already
// claimed by `{n,m}`'s own lazy-suffix parsing (see `bounded_repetition_n_m`,
// `"a{3,5}?"`), the same collision `a??` already resolved back in Lesson 5.a
// -- it never reaches this loop as a fresh quantifier to stack.

#[test]
fn bounded_repetition_can_be_stacked_with_other_quantifiers() {
    assert_eq!(ast("a{3,4}+"), plus(br(lit('a'), 3, Some(4), false)));
    assert_eq!(ast("a{3,4}*"), star(br(lit('a'), 3, Some(4), false)));
    // Order reversed: whatever `node` already is gets wrapped -- `{n,m}`
    // doesn't care that its child is already a `Plus`.
    assert_eq!(ast("a+{2,3}"), br(plus(lit('a')), 2, Some(3), false));
    // Two bonded repetitions stacked, same shape as `stacked_stars`.
    assert_eq!(
        ast("a{3,4}{2,5}"),
        br(br(lit('a'), 3, Some(4), false), 2, Some(5), false)
    );
}

#[test]
fn bounded_repetition_followed_by_double_plus_is_stacking_not_possessive() {
    // In PCRE, `X+` immediately after any quantifier is a *possessive*
    // suffix, not a second quantifier -- `a{3,4}+` means "possessive
    // {3,4}", and a second `+` after that has nothing left to attach to
    // (regex101 rejects `a{3,4}++`). This project deliberately doesn't
    // implement possessive quantifiers -- they require discarding a live
    // thread because of an earlier commitment, which breaks the "a thread's
    // fate depends only on (state, position)" invariant the whole simulation
    // relies on, the same reason backreferences are impossible. Since that
    // feature isn't implemented, `+` after a quantifier is never claimed by
    // it -- it's just `Plus` wrapping whatever came before, same as `a++`
    // already means `Plus(Plus(a))`. This is an intentional divergence from
    // PCRE syntax, not a bug to reconcile with it.
    assert_eq!(
        ast("a{3,4}++"),
        plus(plus(br(lit('a'), 3, Some(4), false)))
    );
}
