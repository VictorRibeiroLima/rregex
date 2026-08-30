use crate::regex::Regex;

fn matches(pattern: &str, input: &str) -> bool {
    Regex::compile(pattern).unwrap().full_match(input).unwrap()
}

#[test]
fn empty_pattern_matches_only_the_empty_string() {
    assert!(matches("", ""));
    assert!(!matches("", "a"));
}

#[test]
fn single_literal() {
    assert!(matches("a", "a"));
    assert!(!matches("a", ""));
    assert!(!matches("a", "b"));
    assert!(!matches("a", "aa"));
}

#[test]
fn simple_concat_regex() {
    let regex = Regex::compile("abc").unwrap();
    assert!(regex.full_match("abc").unwrap());
    assert!(!regex.full_match("abcd").unwrap());
    assert!(!regex.full_match("ab").unwrap());
}

#[test]
fn simple_alternation_regex() {
    let regex = Regex::compile("a|b|c").unwrap();
    assert!(regex.full_match("a").unwrap());
    assert!(regex.full_match("b").unwrap());
    assert!(regex.full_match("c").unwrap());
}

#[test]
fn alternation_rejects_non_members() {
    // L(a|b|c) is three strings, all of length 1. Nothing longer can match,
    // no matter which branch it starts with.
    assert!(!matches("a|b|c", "d"));
    assert!(!matches("a|b|c", ""));
    assert!(!matches("a|b|c", "ab"));
}

#[test]
fn empty_alternation_branch() {
    // "a|" parses to Alternation(Literal('a'), Empty) -- the branch that
    // spells nothing is a real branch, so "" is in the language.
    assert!(matches("a|", "a"));
    assert!(matches("a|", ""));
    assert!(!matches("a|", "b"));
}

#[test]
fn plus_matches_one_or_more() {
    assert!(!matches("a+", ""));
    assert!(matches("a+", "a"));
    assert!(matches("a+", "aaaaa"));
    assert!(!matches("a+", "b"));
    assert!(!matches("a+", "ab"));
}

#[test]
fn plus_of_an_alternation() {
    assert!(!matches("(a|b)+", ""));
    assert!(matches("(a|b)+", "a"));
    assert!(matches("(a|b)+", "abbaab"));
    assert!(!matches("(a|b)+", "c"));
    assert!(!matches("(a|b)+", "abc"));
}

#[test]
fn a_plus_followed_by_the_same_literal() {
    assert!(!matches("a+a", "a")); // the mandatory 'a' leaves nothing for the trailing literal
    assert!(matches("a+a", "aa"));
    assert!(matches("a+a", "aaaa"));
    assert!(!matches("a+a", ""));
}

#[test]
fn question_matches_zero_or_one() {
    assert!(matches("a?", ""));
    assert!(matches("a?", "a"));
    assert!(!matches("a?", "aa"));
    assert!(!matches("a?", "b"));
}

#[test]
fn question_of_an_alternation() {
    assert!(matches("(a|b)?", ""));
    assert!(matches("(a|b)?", "a"));
    assert!(matches("(a|b)?", "b"));
    assert!(!matches("(a|b)?", "ab"));
    assert!(!matches("(a|b)?", "c"));
}

#[test]
fn a_question_followed_by_the_same_literal() {
    assert!(matches("a?a", "a"));
    assert!(matches("a?a", "aa"));
    assert!(!matches("a?a", "aaa")); // '?' caps the optional part at one, plus the mandatory one is at most two
    assert!(!matches("a?a", ""));
}

#[test]
fn class_matches_exactly_one_member() {
    assert!(matches("[abc]", "a"));
    assert!(!matches("[abc]", "d"));
    assert!(!matches("[abc]", "")); // a class still consumes exactly one char
    assert!(!matches("[abc]", "ab"));
}

#[test]
fn negated_class_matches_the_complement() {
    assert!(matches("[^abc]", "d"));
    assert!(!matches("[^abc]", "a"));
    assert!(!matches("[^abc]", "b"));
    assert!(!matches("[^abc]", "c"));
}

#[test]
fn class_range_matches_by_scalar_value() {
    assert!(matches("[a-z]", "m"));
    assert!(!matches("[a-z]", "M"));
}

#[test]
fn class_composed_with_plus() {
    assert!(matches("[a-z]+", "hello"));
    assert!(!matches("[a-z]+", "Hello"));
    assert!(!matches("[a-z]+", ""));
}

#[test]
fn escaped_metacharacter_matches_only_the_literal() {
    assert!(matches("a\\*b", "a*b"));
    assert!(!matches("a\\*b", "ab")); // '*' is required now, not optional
    assert!(!matches("a\\*b", "a**b"));
}

#[test]
fn dangling_escape_fails_to_compile() {
    assert!(Regex::compile("a\\").is_err());
}

#[test]
fn a_real_identifier_matcher() {
    // Everything from this lesson working together on an actually-useful
    // pattern -- the same rule every programming language uses to recognize
    // a variable/function name: letters or '_' first, then any run of
    // letters/digits/'_'.
    let identifier = "[a-zA-Z_][a-zA-Z0-9_]*";
    assert!(matches(identifier, "myVariable"));
    assert!(matches(identifier, "_private"));
    assert!(matches(identifier, "x1"));
    assert!(!matches(identifier, "1x")); // can't start with a digit
    assert!(!matches(identifier, "my-variable")); // hyphen isn't in the class
    assert!(matches(identifier, "a"));
    assert!(matches(
        identifier,
        "a_very_long_identifier_name_0123456789_abcdefghijklmnopqrstuvwxyz_ABCDEFGHIJKLMNOPQRSTUVWXYZ_end"
    ));
}

#[test]
fn star_matches_zero_or_more() {
    assert!(matches("a*", ""));
    assert!(matches("a*", "a"));
    assert!(matches("a*", "aaaaa"));
    assert!(!matches("a*", "b"));
    assert!(!matches("a*", "ab"));
}

#[test]
fn star_of_an_alternation() {
    assert!(matches("(a|b)*", ""));
    assert!(matches("(a|b)*", "a"));
    assert!(matches("(a|b)*", "abbaab"));
    assert!(!matches("(a|b)*", "c"));
    assert!(!matches("(a|b)*", "abc"));
}

#[test]
fn star_of_an_alternation_then_a_literal() {
    // The Lesson 2 machine, now run. Ten states, start at 6.
    assert!(matches("(a|b)*c", "c"));
    assert!(matches("(a|b)*c", "ac"));
    assert!(matches("(a|b)*c", "bbabc"));
    assert!(!matches("(a|b)*c", ""));
    assert!(!matches("(a|b)*c", "ab"));
    assert!(!matches("(a|b)*c", "cc"));
}

#[test]
fn concatenated_stars() {
    assert!(matches("a*b*", ""));
    assert!(matches("a*b*", "a"));
    assert!(matches("a*b*", "b"));
    assert!(matches("a*b*", "aaabbb"));
    assert!(!matches("a*b*", "ba"));
}

#[test]
fn a_star_followed_by_the_same_literal() {
    // The set has to keep the "stop looping now" finger alive alongside the
    // "keep looping" one; a greedy walker that commits to the loop never
    // reaches the trailing 'a'.
    assert!(matches("a*a", "a"));
    assert!(matches("a*a", "aa"));
    assert!(matches("a*a", "aaaa"));
    assert!(!matches("a*a", ""));
}

#[test]
fn stacked_stars_terminate() {
    // The epsilon loop from Lesson 2: 4 -> 2 -> 3 -> 4 reads no input.
    // If closure does not remember what it has already added, this hangs
    // or blows the stack instead of failing.
    assert!(matches("a**", ""));
    assert!(matches("a**", "a"));
    assert!(matches("a**", "aaa"));
    assert!(!matches("a**", "b"));
}

#[test]
fn dot_matches_any_single_character() {
    assert!(matches(".", "a"));
    assert!(matches(".", "1"));
    assert!(matches(".", "!"));
    assert!(!matches(".", ""));
    assert!(!matches(".", "ab"));
}

#[test]
fn dot_has_no_special_case_for_whitespace_or_newline() {
    // Unlike most engines' default mode, this alphabet is plain `char` with
    // no carve-outs -- ConsumeAny has no comparison to skip, so every char
    // is a member, newline included.
    assert!(matches(".", " "));
    assert!(matches(".", "\n"));
}

#[test]
fn dot_combined_with_literals_and_star() {
    assert!(matches("a.c", "abc"));
    assert!(!matches("a.c", "ac"));
    assert!(!matches("a.c", "abbc"));
    assert!(matches(".*", ""));
    assert!(matches(".*", "abc123!@#"));
}

#[test]
fn lazy_quantifiers_still_answer_membership_when_the_minimal_path_spans_the_input() {
    // full_match asks "does any accepting path exist" -- when the shortest
    // (lazy, highest-priority) path already happens to consume the whole
    // string, there's nothing shorter for it to be cut in favor of, so this
    // works fine regardless of greedy vs lazy.
    assert!(matches("a*?", ""));
    assert!(matches("a+?", "a"));
    assert!(matches("a??", ""));
    assert!(!matches("a??", "aa")); // '?' still caps at one repetition, lazy or not
}

#[test]
fn lazy_quantifiers_expose_the_same_full_match_bug() {
    // Same root cause as a_higher_priority_short_branch_must_not_hide_a_full_match,
    // reached through laziness instead of alternation order. "a" IS in L(a?) --
    // taking the 'a' is optional, not forbidden -- but the lazy interpretation's
    // highest-priority path is the empty one, find records and cuts on that, and
    // full_match (derived from find) inherits the wrong answer. Same bug,
    // same "do not fix unprompted" as its sibling test.
    assert!(matches("a??", "a"));
    assert!(matches("a+?", "aaa"));
    assert!(matches("a*?", "aaa"));
}

#[test]
fn the_whole_string_must_be_consumed() {
    // full_match answers membership, not search. A pattern occurring inside
    // the input is not a match.
    assert!(!matches("ab", "xab"));
    assert!(!matches("ab", "abx"));
    assert!(!matches("a|b", "ab"));
}

#[test]
fn a_higher_priority_short_branch_must_not_hide_a_full_match() {
    // full_match asks whether the string is in the language -- existential, any
    // accepting path will do. find asks where the *highest-priority* match ends,
    // and cuts every lower-ranked thread the moment it reads a Match.
    //
    // "ab" is in L(a|ab) via the right branch, but at position 1 the list is
    // [Match, Consume('b')]: Match is rank 0, so it is recorded as Some(1) and
    // the thread that would have reached Some(2) is cut. Deriving full_match
    // from find is only sound for leftmost-longest, not leftmost-first.
    assert!(matches("a|ab", "ab"));
    assert!(matches("a|ab|abc", "abc"));
    assert!(matches("a*|b", "b"));

    // The reverse ordering already works, which is why this went unnoticed.
    assert!(matches("ab|a", "ab"));
}
