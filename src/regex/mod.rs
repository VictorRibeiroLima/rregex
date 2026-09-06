use crate::{
    machine::{Instruction, Machine, State, position::Position},
    parser::parse,
    regex::error::RegexError,
};

pub mod error;

#[derive(Debug)]
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

    fn clear(&mut self) {
        self.seen.fill(false);
        self.traversed.clear();
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
        let len = input.chars().count();
        let mut i = 0;
        let mut seen_set = SeenSet::new(self.len());
        //The buffer to move the memory in and out of places.
        //Before this exited, each call to step and closure would allocate memory.
        //The implementation of this resulted in a 40% speedup in step and 34% speedup in closure.
        let mut buffer = SeenSet::new(self.len());
        seen_set.insert(self.machine.start());
        seen_set.traverse(self.machine.start());
        let mut seen_set = self.closure(0, len, seen_set, &mut buffer);
        for c in input.chars() {
            let position = i + 1; //Length based position
            let matched = self.step(&seen_set, &mut buffer, c);
            std::mem::swap(&mut seen_set, &mut buffer);
            if matched {
                result = Some(i);
            }
            if seen_set.traversed.is_empty() {
                break;
            }
            seen_set = self.closure(position, len, seen_set, &mut buffer);
            i = position;
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

    fn step(&self, seen_set: &SeenSet, next_buffer: &mut SeenSet, c: char) -> bool {
        next_buffer.clear();
        let program = self.machine.program();
        let traversed = &seen_set.traversed;
        for i in traversed {
            let inst = &program[*i];
            match inst {
                Instruction::Consume(t, j) => {
                    if *t != c {
                        continue;
                    }
                    if next_buffer.insert(*j) {
                        next_buffer.traverse(*j);
                    }
                }
                Instruction::ConsumeAny(j) => {
                    if next_buffer.insert(*j) {
                        next_buffer.traverse(*j);
                    }
                }
                Instruction::ConsumeClass(class) => {
                    if class.match_c(c) {
                        let j = class.exit();
                        if next_buffer.insert(j) {
                            next_buffer.traverse(j);
                        }
                    }
                }
                Instruction::Match => {
                    return true;
                }
                _ => continue,
            };
        }
        return false;
    }

    fn closure(
        &self,
        position: usize,
        len: usize,
        mut seen_set: SeenSet,
        buffer: &mut SeenSet,
    ) -> SeenSet {
        buffer.clear();
        std::mem::swap(&mut buffer.traversed, &mut seen_set.traversed);
        let traversed = &buffer.traversed;
        for i in traversed {
            self.follow(&mut seen_set, *i, position, len);
        }
        seen_set
    }

    fn follow(&self, seen_set: &mut SeenSet, i: usize, position: usize, len: usize) {
        let program = self.machine.program();
        let inst = &program[i];
        match inst {
            Instruction::Jump(j) => {
                if !seen_set.insert(*j) {
                    return;
                }
                self.follow(seen_set, *j, position, len);
            }
            Instruction::Split(j1, j2) => {
                if seen_set.insert(*j1) {
                    self.follow(seen_set, *j1, position, len);
                }
                if seen_set.insert(*j2) {
                    self.follow(seen_set, *j2, position, len);
                }
            }
            Instruction::ConditionalJump(p, j) => {
                match p {
                    Position::Start => {
                        if position != 0 {
                            return;
                        }
                    }
                    Position::End => {
                        if position != len {
                            return;
                        }
                    }
                };
                if seen_set.insert(*j) {
                    self.follow(seen_set, *j, position, len);
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
            let inst = &program[*i];
            if let Instruction::Match = inst {
                return true;
            }
        }
        return false;
    }
}

#[cfg(test)]
mod tests;
