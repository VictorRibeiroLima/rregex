use crate::{
    machine::{Instruction, Machine, State},
    parser::parse,
    regex::error::RegexError,
};

pub mod error;

struct SeenSet {
    seen: Vec<bool>,
    traversed: Vec<State>,
}

impl SeenSet {
    fn new(n: usize) -> Self {
        let seen = vec![false; n];
        SeenSet {
            seen,
            traversed: Vec::new(),
        }
    }

    fn insert(&mut self, state: State) -> bool {
        if self.seen[state] {
            return false;
        }
        self.seen[state] = true;
        return true;
    }

    fn traverse(&mut self, state: State) {
        self.traversed.push(state);
    }

    fn rebuild(mut self) -> (Self, Vec<State>) {
        let traversed = std::mem::take(&mut self.traversed);
        (self, traversed)
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

    pub fn find(&self, input: &str) -> Result<Option<usize>, RegexError> {
        let mut result = None;
        let mut i = 0;
        let mut seen_set = SeenSet::new(self.len());
        seen_set.insert(self.machine.start());
        seen_set.traverse(self.machine.start());
        let mut seen_set = self.closure(seen_set);
        for c in input.chars() {
            let (next_seen_set, matched) = self.step(&seen_set, c);
            seen_set = next_seen_set;
            if matched {
                result = Some(i);
            }
            if seen_set.traversed.is_empty() {
                break;
            }
            seen_set = self.closure(seen_set);
            i += 1;
        }
        let matched = self.is_match(&seen_set);
        if matched {
            result = Some(i);
        }
        return Ok(result);
    }

    pub fn full_match(&self, input: &str) -> Result<bool, RegexError> {
        let input_len = input.chars().count();
        let result = self.find(input)?;
        return Ok(result == Some(input_len));
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.machine.program().len()
    }

    fn step(&self, seen_set: &SeenSet, c: char) -> (SeenSet, bool) {
        let mut next = SeenSet::new(self.len());
        let program = self.machine.program();
        let traversed = &seen_set.traversed;
        for i in traversed {
            let inst = &program[*i];
            match inst {
                Instruction::Consume(t, j) => {
                    if *t != c {
                        continue;
                    }
                    if next.insert(*j) {
                        next.traverse(*j);
                    }
                }
                Instruction::ConsumeAny(j) => {
                    if next.insert(*j) {
                        next.traverse(*j);
                    }
                }
                Instruction::ConsumeClass(class) => {
                    if class.match_c(c) {
                        let j = class.exit();
                        if next.insert(j) {
                            next.traverse(j);
                        }
                    }
                }
                Instruction::Match => {
                    return (next, true);
                }
                _ => continue,
            };
        }
        return (next, false);
    }

    fn closure(&self, seen_set: SeenSet) -> SeenSet {
        let (mut seen_set, traversed) = seen_set.rebuild();
        for i in traversed {
            self.follow(&mut seen_set, i);
        }
        seen_set
    }

    fn follow(&self, seen_set: &mut SeenSet, i: usize) {
        let program = self.machine.program();
        let inst = &program[i];
        match inst {
            Instruction::Jump(j) => {
                if !seen_set.insert(*j) {
                    return;
                }
                self.follow(seen_set, *j);
            }
            Instruction::Split(j1, j2) => {
                if seen_set.insert(*j1) {
                    self.follow(seen_set, *j1);
                }
                if seen_set.insert(*j2) {
                    self.follow(seen_set, *j2);
                }
            }
            Instruction::Consume(_, _)
            | Instruction::Match
            | Instruction::ConsumeAny(_)
            | Instruction::ConsumeClass(_) => {
                seen_set.traverse(i);
            }
        };
    }

    fn is_match(&self, seen_set: &SeenSet) -> bool {
        let program = self.machine.program();
        for i in &seen_set.traversed {
            if let Instruction::Match = program[*i] {
                return true;
            }
        }
        return false;
    }
}

#[cfg(test)]
mod tests;
