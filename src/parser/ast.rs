use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Eq)]
pub enum Ast {
    Empty,
    Literal(char),
    Concat(Box<Ast>, Box<Ast>),
    Alternation(Box<Ast>, Box<Ast>),
    Star(Box<Ast>),
    LazyStar(Box<Ast>),
    Plus(Box<Ast>),
    LazyPlus(Box<Ast>),
    Question(Box<Ast>),
    LazyQuestion(Box<Ast>),
    Class(ClassSet, bool), // bool indicates negation
    Any,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClassType {
    Range(char, char),
    Single(char),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ClassSet(pub Vec<ClassType>);

impl ClassSet {
    pub fn new() -> Self {
        ClassSet(Vec::new())
    }

    #[cfg(test)]
    pub fn from_vec(vec: Vec<ClassType>) -> Self {
        ClassSet(vec)
    }

    pub fn push(&mut self, class_type: ClassType) {
        if !self.0.contains(&class_type) {
            self.0.push(class_type);
        }
    }
}

impl Deref for ClassSet {
    type Target = Vec<ClassType>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ClassSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
