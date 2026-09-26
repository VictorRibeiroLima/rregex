use super::*;

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
