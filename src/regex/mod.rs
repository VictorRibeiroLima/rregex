use crate::{machine::Machine, parser::parse};

pub struct Regex {
    machine: Machine,
}

impl Regex {
    pub fn compile(pattern: &str) -> Result<Self, String> {
        let ast = parse(pattern).map_err(|_| format!("TODO:error"))?;
        let machine = Machine::new(ast);
        Ok(Regex { machine })
    }

    pub fn matches(&self, input: &str) -> bool {
        let mut seen_set: Vec<bool> = self.init_seen_set();
        todo!()
    }

    fn init_seen_set(&self) -> Vec<bool> {
        let mut seen_set: Vec<bool> = vec![false; self.machine.program().len()];
        for inst in self.machine.program().iter() {
            match inst {
                crate::machine::ValidInstruction::Consume(_, _) => todo!(),
                crate::machine::ValidInstruction::Jump(_) => todo!(),
                crate::machine::ValidInstruction::Split(_, _) => todo!(),
                crate::machine::ValidInstruction::Match => todo!(),
            }
        }

        todo!()
    }
}
