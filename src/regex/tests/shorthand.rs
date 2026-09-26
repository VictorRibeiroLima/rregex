use crate::regex::Regex;

// End to end: `\d \w \s \D \W \S \n \t \r` outside a class, through parse ->
// compile -> simulate. ASCII definitions: `\w` is [a-zA-Z0-9_].

fn find(pattern: &str, input: &str) -> Option<usize> {
    Regex::compile(pattern).unwrap().find(input).unwrap()
}

fn matches(pattern: &str, input: &str) -> bool {
    Regex::compile(pattern).unwrap().full_match(input).unwrap()
}

#[test]
fn digit_boundaries() {
    for c in ['0', '5', '9'] {
        assert!(matches("\\d", &c.to_string()), "\\d should match {c:?}");
        assert!(!matches("\\D", &c.to_string()), "\\D should reject {c:?}");
    }
    // '/' and ':' sit on either side of '0'..'9'.
    for c in ['/', ':', 'a', ' ', '\u{10FFFF}'] {
        assert!(!matches("\\d", &c.to_string()), "\\d should reject {c:?}");
        assert!(matches("\\D", &c.to_string()), "\\D should match {c:?}");
    }
}

#[test]
fn word_boundaries() {
    for c in ['a', 'z', 'A', 'Z', '0', '9', '_'] {
        assert!(matches("\\w", &c.to_string()), "\\w should match {c:?}");
        assert!(!matches("\\W", &c.to_string()), "\\W should reject {c:?}");
    }
    // The neighbours of every range edge, plus non-ASCII letters (ASCII \w).
    for c in ['/', ':', '@', '[', '^', '`', '{', ' ', '-', 'é', '\u{10FFFF}'] {
        assert!(!matches("\\w", &c.to_string()), "\\w should reject {c:?}");
        assert!(matches("\\W", &c.to_string()), "\\W should match {c:?}");
    }
}

#[test]
fn space_boundaries() {
    for c in [' ', '\t', '\n', '\x0B', '\x0C', '\r'] {
        assert!(matches("\\s", &c.to_string()), "\\s should match {c:?}");
        assert!(!matches("\\S", &c.to_string()), "\\S should reject {c:?}");
    }
    // Just outside '\t'..'\r' and on either side of ' '.
    for c in ['\x08', '\x0E', '\x1F', '!', 'a', '\0', '\u{10FFFF}'] {
        assert!(!matches("\\s", &c.to_string()), "\\s should reject {c:?}");
        assert!(matches("\\S", &c.to_string()), "\\S should match {c:?}");
    }
}

#[test]
fn a_shorthand_escape_matches_exactly_one_char() {
    assert!(!matches("\\d", ""));
    assert!(!matches("\\d", "12"));
    assert!(!matches("\\S", "ab"));
}

#[test]
fn quantified_shorthands() {
    assert!(matches("\\d+", "2026"));
    assert!(!matches("\\d+", ""));
    assert_eq!(find("\\d+", "20a6"), Some(2));
    assert_eq!(find("\\S+", "abc def"), Some(3));
    assert!(matches("\\s*", ""));
    assert!(matches("\\w{3}", "a_1"));
    assert!(!matches("\\w{3}", "a_"));
}

#[test]
fn lazy_shorthands_stop_as_early_as_they_can() {
    assert_eq!(find("\\d+?", "123"), Some(1));
    assert_eq!(find("\\d*?", "123"), Some(0));
    // The lazy loop still has to reach the `\s` that follows it.
    assert_eq!(find("\\d*?\\s", "12 3"), Some(3));
}

#[test]
fn shorthands_compose_into_realistic_patterns() {
    // A phone-number shape.
    assert!(matches("\\d{3}-\\d{4}", "555-1234"));
    assert!(!matches("\\d{3}-\\d{4}", "555-123"));
    assert!(!matches("\\d{3}-\\d{4}", "5551234"));
    assert!(!matches("\\d{3}-\\d{4}", "55a-1234"));

    // Two words separated by exactly one whitespace char.
    assert!(matches("\\w+\\s\\w+", "hello world"));
    assert!(matches("\\w+\\s\\w+", "hello\tworld"));
    assert!(!matches("\\w+\\s\\w+", "hello  world"));
    assert!(!matches("\\w+\\s\\w+", "hello-world"));

    // A key=value pair with optional spaces around `=`.
    assert!(matches("\\w+\\s*=\\s*\\w+", "max_len = 42"));
    assert!(matches("\\w+\\s*=\\s*\\w+", "max_len=42"));
    assert!(!matches("\\w+\\s*=\\s*\\w+", "max len=42"));
}

#[test]
fn a_class_and_its_complement_split_every_input() {
    // Every char is in exactly one of \d and \D, so (\d|\D)* is `.*`.
    assert!(matches("(\\d|\\D)*", "a1 _\t\u{10FFFF}"));
    assert!(matches("(\\w|\\W)*", "a1 _\t\u{10FFFF}"));
    assert!(matches("(\\s|\\S)*", "a1 _\t\u{10FFFF}"));
    // And they alternate cleanly.
    assert!(matches("(\\d\\D)*", "1a2b3c"));
    assert!(!matches("(\\d\\D)*", "1a22"));
}

#[test]
fn control_char_escapes_match_the_control_char_not_the_letter() {
    assert!(matches("a\\nb", "a\nb"));
    assert!(!matches("a\\nb", "anb"));
    assert!(matches("\\t", "\t"));
    assert!(!matches("\\t", "t"));
    assert!(matches("\\r\\n", "\r\n"));
    // `\n` and `\t` are also members of `\s`.
    assert!(matches("\\s\\s", "\n\t"));
}

#[test]
fn an_escaped_backslash_followed_by_a_letter_is_two_chars() {
    assert!(matches("\\\\n", "\\n"));
    assert!(!matches("\\\\n", "\n"));
    assert!(matches("\\\\d", "\\d"));
    assert!(!matches("\\\\d", "5"));
}
