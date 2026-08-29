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
