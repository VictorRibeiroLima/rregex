use crate::regex::Regex;

fn find(pattern: &str, input: &str) -> Option<usize> {
    Regex::compile(pattern).unwrap().find(input).unwrap()
}

#[test]
fn find_reports_how_far_the_pattern_got() {
    // Some(n) means the match is input[0..n]. Leftover input is not failure.
    assert_eq!(find("ab", "ab"), Some(2));
    assert_eq!(find("ab", "abc"), Some(2));
    assert_eq!(find("ab", "a"), None);
    assert_eq!(find("ab", "xab"), None); // still anchored at 0
}

#[test]
fn zero_length_matches_are_real_matches() {
    // Some(0) is a match, and it is not the same answer as None.
    assert_eq!(find("", ""), Some(0));
    assert_eq!(find("", "abc"), Some(0));
    assert_eq!(find("a*", "b"), Some(0));
    assert_eq!(find("a", "b"), None);
}

#[test]
fn a_match_ending_at_end_of_input_is_still_seen() {
    // The final list has to be read after the loop; no character arrives to
    // trigger the read.
    assert_eq!(find("a", "a"), Some(1));
    assert_eq!(find("abc", "abc"), Some(3));
    assert_eq!(find("a|b", "b"), Some(1));
}

#[test]
fn alternation_prefers_the_left_branch() {
    // Same language, one Split's fields exchanged. This is the whole lesson.
    assert_eq!(find("a|ab", "ab"), Some(1));
    assert_eq!(find("ab|a", "ab"), Some(2));
}

#[test]
fn an_empty_branch_still_loses_to_a_live_thread() {
    // "a|" -> the empty branch reaches Match at position 0, but the 'a'
    // thread outranks it and overwrites the recorded offset.
    assert_eq!(find("a|", "a"), Some(1));
    assert_eq!(find("a|", "b"), Some(0));
}

#[test]
fn star_is_greedy() {
    // The loop thread sits above Match at every position, so the recorded
    // offset climbs instead of freezing at 0.
    assert_eq!(find("a*", ""), Some(0));
    assert_eq!(find("a*", "aaa"), Some(3));
    assert_eq!(find("a*", "ab"), Some(1));
    assert_eq!(find("a**", "aaa"), Some(3));
}

#[test]
fn the_loop_thread_and_the_exit_thread_both_stay_alive() {
    // a*a has no backtracking to fall back on: the "stop looping" thread
    // must already be in the list when the trailing 'a' arrives.
    assert_eq!(find("a*a", "a"), Some(1));
    assert_eq!(find("a*a", "aa"), Some(2));
    assert_eq!(find("a*a", "aaaa"), Some(4));
    assert_eq!(find("a*a", ""), None);
}

#[test]
fn dot_consumes_exactly_one_character() {
    assert_eq!(find(".", "a"), Some(1));
    assert_eq!(find(".", ""), None);
    assert_eq!(find(".", "ab"), Some(1)); // leftover 'b' is fine, find isn't anchored at the end
}

#[test]
fn dot_star_is_greedy_over_any_character() {
    assert_eq!(find(".*", "abc123!@#"), Some(9));
    assert_eq!(find(".*", ""), Some(0));
}

#[test]
fn dot_between_literals_requires_exactly_one_character() {
    assert_eq!(find("a.c", "abc"), Some(3));
    assert_eq!(find("a.c", "ac"), None); // nothing there for '.' to consume
    assert_eq!(find("a.c", "abbc"), None); // one char too many between a and c
}

#[test]
fn find_through_a_star_of_an_alternation() {
    assert_eq!(find("(a|b)*c", "c"), Some(1));
    assert_eq!(find("(a|b)*c", "bbabc"), Some(5));
    assert_eq!(find("(a|b)*c", "abcd"), Some(3));
    assert_eq!(find("(a|b)*c", "ab"), None);
}
