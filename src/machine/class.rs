use crate::{machine::State, parser::ast::ClassType};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Class {
    instructions: Vec<ClassInstruction>,
    negated: bool,
    exit: State,
}

impl Class {
    pub fn new(instructions: Vec<ClassInstruction>, negated: bool, exit: State) -> Self {
        Class {
            instructions,
            negated,
            exit,
        }
    }

    pub fn exit(&self) -> State {
        self.exit
    }

    pub fn match_c(&self, c: char) -> bool {
        let mut matched = false;
        for instruction in &self.instructions {
            match instruction {
                ClassInstruction::Range(start, end) => {
                    if *start <= c && c <= *end {
                        matched = true;
                        break;
                    }
                }
                ClassInstruction::Single(ch) => {
                    if *ch == c {
                        matched = true;
                        break;
                    }
                }
            }
        }
        if self.negated { !matched } else { matched }
    }
}

impl From<&ClassType> for ClassInstruction {
    fn from(class: &ClassType) -> Self {
        match class {
            ClassType::Range(start, end) => ClassInstruction::Range(*start, *end),
            ClassType::Single(c) => ClassInstruction::Single(*c),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClassInstruction {
    Range(char, char),
    Single(char),
}
