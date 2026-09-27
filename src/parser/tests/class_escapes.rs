use super::*;

// Escapes inside `[...]` follow the same rule as parse_atom's `\` branch:
// whatever follows `\` is a literal. The "Actual:" notes in the first four
// tests describe the parser before it learned that rule -- each one is a
// distinct way a class parser without escape handling goes wrong.

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
    // Intended: `[a-\{]` is the range 'a'..'{' (0x61..0x7B).
    // Actual: the range logic sees `-` followed by a char and takes that char
    // as the endpoint, but the char it finds is the backslash, so it builds
    // 'a'..'\\' -- inverted, because '\\' (0x5C) < 'a' (0x61) -- and reports
    // InvalidRange for a range the pattern never wrote.
    //
    // (Not `[a-\]]`: ']' is 0x5D, so 'a'..']' is inverted too and
    // InvalidRange would be the *correct* answer.)
    //
    // The order this implies: an escape has to resolve to its char *before*
    // range detection runs, not after. Note that resolving it isn't enough on
    // its own -- once `\d` exists, an escape can resolve to a whole class,
    // which is not a legal endpoint at all.
    assert_eq!(
        ast("[a-\\{]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Range('a', '{')]), false)
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

// --- class escapes: edge cases ----------------------------------------------

#[test]
fn a_dangling_backslash_inside_a_class_is_unexpected_end_of_input() {
    // A `\` with nothing after it escapes nothing -- the same rule parse_atom
    // applies outside a class (`a\` is UnexpectedEndOfInput). The first two
    // only pass by accident today: peek_escaped reports a lone `\` as
    // Some('\\'), it gets pushed as a member, and *then* EOF is hit.
    assert!(matches!(
        parse("[\\"),
        Err(ParserError::UnexpectedEndOfInput)
    ));
    assert!(matches!(
        parse("[a\\"),
        Err(ParserError::UnexpectedEndOfInput)
    ));
    // Here the accident runs out: the disguised backslash becomes the range's
    // right endpoint, and 'a' (0x61) > '\\' (0x5C) reports InvalidRange for a
    // range the pattern never finished writing.
    assert!(matches!(
        parse("[a-\\"),
        Err(ParserError::UnexpectedEndOfInput)
    ));
}

#[test]
fn an_escape_can_be_the_left_endpoint_of_a_range() {
    assert_eq!(
        ast("[\\]-a]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Range(']', 'a')]), false)
    );
    assert_eq!(
        ast("[\\\\-a]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Range('\\', 'a')]), false)
    );
}

#[test]
fn an_escaped_dash_is_an_endpoint_never_the_range_operator() {
    // As the right endpoint: '!'..'-' (0x21..0x2D).
    assert_eq!(
        ast("[!-\\-]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Range('!', '-')]), false)
    );
    // Between two chars: three members, not 'a'..'z'.
    assert_eq!(
        ast("[a\\-z]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Single('a'),
                ClassType::Single('-'),
                ClassType::Single('z'),
            ]),
            false
        )
    );
}

#[test]
fn only_an_unescaped_leading_caret_negates() {
    // The first `^` negates; the escaped one is the class's only member.
    assert_eq!(
        ast("[^\\^]"),
        Ast::Class(ClassSet::from_vec(vec![ClassType::Single('^')]), true)
    );
}

#[test]
fn control_char_escapes_can_be_range_endpoints() {
    // `\t` and `\r` each resolve to one char, so unlike `\d` they are legal
    // endpoints: '\t'..'\r' is 0x09..0x0D, the contiguous whitespace block
    // `\s` is built from. The naive "whatever follows `\` is a literal" rule
    // reads this as 't'..'r' instead -- inverted, since 't' (0x74) > 'r'
    // (0x72) -- so it fails with InvalidRange until the escapes mean control
    // chars.
    assert_eq!(
        ast("[\\t-\\r]"),
        Ast::Class(
            ClassSet::from_vec(vec![ClassType::Range('\t', '\r')]),
            false
        )
    );
}

#[test]
fn a_shorthand_on_the_left_of_a_dash_makes_the_dash_literal() {
    // `\d` is a set, not a char, so it can't open a range. The `-` falls back
    // to a literal member, same as after a finished range (`[a-c-e]`).
    assert_eq!(
        ast("[\\d-a]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Range('0', '9'),
                ClassType::Single('-'),
                ClassType::Single('a'),
            ]),
            false
        )
    );
}

#[test]
fn a_shorthand_on_the_right_of_a_dash_makes_the_dash_literal() {
    // The mirror image: `\d` can't close a range either. Nothing to the left
    // of the `-` may be lost along the way -- `a` is still a member.
    assert_eq!(
        ast("[a-\\d]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Single('a'),
                ClassType::Single('-'),
                ClassType::Range('0', '9'),
            ]),
            false
        )
    );
}

#[test]
fn a_caret_after_a_leading_escape_is_a_literal_member() {
    // `^` only negates as the very first thing in the class. An escape --
    // shorthand or not -- in first position used up that slot, so a
    // following `^` is just a member.
    assert_eq!(
        ast("[\\d^]"),
        Ast::Class(
            ClassSet::from_vec(vec![ClassType::Range('0', '9'), ClassType::Single('^')]),
            false
        )
    );
    assert_eq!(
        ast("[\\n^]"),
        Ast::Class(
            ClassSet::from_vec(vec![ClassType::Single('\n'), ClassType::Single('^')]),
            false
        )
    );
}

#[test]
fn shorthand_entries_are_deduplicated_like_any_other() {
    // ClassSet never holds the same entry twice, whichever path pushed it.
    let digit = Ast::Class(ClassSet::from_vec(vec![ClassType::Range('0', '9')]), false);
    assert_eq!(ast("[\\d\\d]"), digit);
    assert_eq!(ast("[0-9\\d]"), digit);
    assert_eq!(ast("[\\d0-9]"), digit);
    // `\w` already contains '0'..'9', so the `\d` after it adds nothing.
    assert_eq!(
        ast("[\\w\\d]"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Range('a', 'z'),
                ClassType::Range('A', 'Z'),
                ClassType::Range('0', '9'),
                ClassType::Single('_'),
            ]),
            false
        )
    );
}
