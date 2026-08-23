use crate::{
    machine::{Instruction, Machine},
    parser::parse,
    regex::error::RegexError,
};
use std::ops::{Deref, DerefMut};

pub mod error;

struct SeenSet {
    seen: Vec<bool>,
    matched: bool,
}

impl SeenSet {
    fn new(n: usize) -> Self {
        let seen = vec![false; n];
        SeenSet {
            seen,
            matched: false,
        }
    }
}
impl Deref for SeenSet {
    type Target = Vec<bool>;

    fn deref(&self) -> &Self::Target {
        &self.seen
    }
}

impl DerefMut for SeenSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.seen
    }
}

pub struct Regex {
    machine: Machine,
}

impl Regex {
    pub fn compile(pattern: &str) -> Result<Self, RegexError> {
        let ast = parse(pattern)?;
        let machine = Machine::new(ast);
        Ok(Regex { machine })
    }

    pub fn matches(&self, input: &str) -> Result<bool, RegexError> {
        let mut seen_set = SeenSet::new(self.len());
        seen_set[self.machine.start()] = true;
        let mut seen_set = self.closure(seen_set);
        for c in input.chars() {
            seen_set = self.step(&seen_set, c);
            seen_set = self.closure(seen_set);
        }
        return Ok(seen_set.matched);
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.machine.program().len()
    }

    fn step(&self, seen_set: &SeenSet, c: char) -> SeenSet {
        let mut next = SeenSet::new(self.len());
        let program = self.machine.program();
        for (i, seen) in seen_set.iter().enumerate() {
            if !seen {
                continue;
            }
            let inst = program[i];
            match inst {
                Instruction::Consume(t, j) => {
                    if t == c {
                        next[j] = true;
                    }
                }
                _ => continue,
            };
        }
        return next;
    }

    fn closure(&self, mut seen_set: SeenSet) -> SeenSet {
        for i in 0..self.len() {
            let seen = seen_set[i];
            if !seen {
                continue;
            }
            self.follow(&mut seen_set, i);
        }
        seen_set
    }

    fn follow(&self, seen_set: &mut SeenSet, i: usize) {
        let program = self.machine.program();
        let inst = program[i];
        match inst {
            Instruction::Jump(j) => {
                if seen_set[j] {
                    return;
                }
                seen_set[j] = true;
                self.follow(seen_set, j);
            }
            Instruction::Split(j1, j2) => {
                if !seen_set[j1] {
                    seen_set[j1] = true;
                    self.follow(seen_set, j1);
                }
                if !seen_set[j2] {
                    seen_set[j2] = true;
                    self.follow(seen_set, j2);
                }
            }
            Instruction::Match => {
                seen_set.matched = true;
            }
            _ => {}
        };
    }
}

#[cfg(test)]
mod test {
    use crate::regex::Regex;

    fn matches(pattern: &str, input: &str) -> bool {
        Regex::compile(pattern).unwrap().matches(input).unwrap()
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
        assert!(regex.matches("abc").unwrap());
        assert!(!regex.matches("abcd").unwrap());
        assert!(!regex.matches("ab").unwrap());
    }

    #[test]
    fn simple_alternation_regex() {
        let regex = Regex::compile("a|b|c").unwrap();
        assert!(regex.matches("a").unwrap());
        assert!(regex.matches("b").unwrap());
        assert!(regex.matches("c").unwrap());
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
    fn the_whole_string_must_be_consumed() {
        // Machine answers membership, not search. A pattern occurring inside
        // the input is not a match.
        assert!(!matches("ab", "xab"));
        assert!(!matches("ab", "abx"));
        assert!(!matches("a|b", "ab"));
    }
}
