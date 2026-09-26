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

// --- shorthand escapes outside a class --------------------------------------
//
// Entries are spelled out rather than taken from `consts.rs`, so a wrong
// constant fails here instead of being compared against itself. Their order
// is whatever parse_slash pushes; membership doesn't depend on it.

fn digit() -> Ast {
    Ast::Class(ClassSet::from_vec(vec![ClassType::Range('0', '9')]), false)
}

fn word() -> Ast {
    Ast::Class(
        ClassSet::from_vec(vec![
            ClassType::Range('a', 'z'),
            ClassType::Range('A', 'Z'),
            ClassType::Range('0', '9'),
            ClassType::Single('_'),
        ]),
        false,
    )
}

fn space() -> Ast {
    Ast::Class(
        ClassSet::from_vec(vec![ClassType::Range('\t', '\r'), ClassType::Single(' ')]),
        false,
    )
}

#[test]
fn shorthand_escapes_parse_to_their_classes() {
    assert_eq!(ast("\\d"), digit());
    assert_eq!(ast("\\w"), word());
    assert_eq!(ast("\\s"), space());
    assert_eq!(
        ast("\\D"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Range('\0', '/'),
                ClassType::Range(':', '\u{10FFFF}'),
            ]),
            false
        )
    );
    assert_eq!(
        ast("\\W"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Range('\0', '/'),
                ClassType::Range(':', '@'),
                ClassType::Range('[', '^'),
                ClassType::Range('{', '\u{10FFFF}'),
                ClassType::Single('`'),
            ]),
            false
        )
    );
    assert_eq!(
        ast("\\S"),
        Ast::Class(
            ClassSet::from_vec(vec![
                ClassType::Range('\0', '\x08'),
                ClassType::Range('\x0E', '\x1F'),
                ClassType::Range('!', '\u{10FFFF}'),
            ]),
            false
        )
    );
}

#[test]
fn control_char_escapes_parse_to_the_control_char() {
    assert_eq!(ast("\\n"), lit('\n'));
    assert_eq!(ast("\\t"), lit('\t'));
    assert_eq!(ast("\\r"), lit('\r'));
}

#[test]
fn an_escaped_backslash_does_not_start_a_second_escape() {
    // Pattern chars: `\` `\` `n`. The first `\` escapes the second, so the
    // `n` is a plain letter -- not a newline.
    assert_eq!(ast("\\\\n"), cat(lit('\\'), lit('n')));
    assert_eq!(ast("\\\\d"), cat(lit('\\'), lit('d')));
}

#[test]
fn a_shorthand_escape_is_one_atom() {
    // Quantifiers bind to the whole class, never to the letter after `\`.
    assert_eq!(ast("\\d+"), plus(digit()));
    assert_eq!(ast("\\w*?"), lazy_star(word()));
    assert_eq!(ast("\\s{2,3}"), br(space(), 2, Some(3), false));
    // And it composes like any other atom.
    assert_eq!(ast("a\\db"), cat(lit('a'), cat(digit(), lit('b'))));
    assert_eq!(ast("\\d|\\s"), alt(digit(), space()));
    assert_eq!(ast("(\\d\\w)+"), plus(cat(digit(), word())));
    assert_eq!(ast("\\d\\n"), cat(digit(), lit('\n')));
}
