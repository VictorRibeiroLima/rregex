use std::ops::Deref;

use crate::machine::{State, class::Class};

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    Hole,
    Consume(char, State),
    ConsumeAny(State),
    ConsumeClass(Class),
    Jump(State),
    Split(State, State),
    Match,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValidInstruction {
    Consume(char, State),
    ConsumeAny(State),
    ConsumeClass(Class),
    Jump(State),
    Split(State, State),
    Match,
}

pub type Program = Vec<Instruction>;

pub struct ValidProgram {
    program: Vec<ValidInstruction>,
}

impl ValidProgram {
    pub fn new(program: Program) -> Result<Self, String> {
        let mut valid_program = Vec::new();
        for inst in program {
            match inst {
                Instruction::Hole => return Err("Program contains a hole".to_string()),
                Instruction::Consume(c, s) => valid_program.push(ValidInstruction::Consume(c, s)),
                Instruction::ConsumeAny(s) => valid_program.push(ValidInstruction::ConsumeAny(s)),
                Instruction::Jump(s) => valid_program.push(ValidInstruction::Jump(s)),
                Instruction::Split(s1, s2) => valid_program.push(ValidInstruction::Split(s1, s2)),
                Instruction::ConsumeClass(class) => {
                    valid_program.push(ValidInstruction::ConsumeClass(class))
                }
                Instruction::Match => valid_program.push(ValidInstruction::Match),
            }
        }
        Ok(ValidProgram {
            program: valid_program,
        })
    }
}

impl Deref for ValidProgram {
    type Target = [ValidInstruction];

    fn deref(&self) -> &Self::Target {
        &self.program
    }
}
