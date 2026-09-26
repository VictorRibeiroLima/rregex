use crate::{
    machine::Machine,
    parser::ast::{Ast, ClassSet, ClassType},
    regex::Regex,
};

// `[\w\s]` built by hand, skipping the parser (which can't produce it yet).
// Checks that a class assembled from several shorthand expansions needs
// nothing new from `machine/` or `regex/`.
//
// \w = [a-zA-Z0-9_]
// \s = [ \t\n\x0B\x0C\r] -- \t..\r are the contiguous code points 0x09..0x0D
fn word_or_space() -> Regex {
    let ast = Ast::Class(
        ClassSet::from_vec(vec![
            ClassType::Range('a', 'z'),
            ClassType::Range('A', 'Z'),
            ClassType::Range('0', '9'),
            ClassType::Single('_'),
            ClassType::Range('\t', '\r'),
            ClassType::Single(' '),
        ]),
        false,
    );
    Regex {
        machine: Machine::new(ast),
    }
}

#[test]
fn a_hand_built_word_or_space_class_matches_exactly_one_member() {
    let regex = word_or_space();

    for c in ['a', 'm', 'z', 'A', 'Q', 'Z', '0', '5', '9', '_'] {
        assert!(regex.full_match(&c.to_string()).unwrap(), "\\w member {c:?}");
    }
    for c in [' ', '\t', '\n', '\x0B', '\x0C', '\r'] {
        assert!(regex.full_match(&c.to_string()).unwrap(), "\\s member {c:?}");
    }

    // Just outside each range's endpoints, plus assorted punctuation.
    for c in ['`', '{', '@', '[', '/', ':', '\x08', '\x0E', '-', '.', '\\', 'é'] {
        assert!(!regex.full_match(&c.to_string()).unwrap(), "non-member {c:?}");
    }

    // A class is one char wide: no match on "" and none on two members.
    assert!(!regex.full_match("").unwrap());
    assert!(!regex.full_match("a ").unwrap());
}
