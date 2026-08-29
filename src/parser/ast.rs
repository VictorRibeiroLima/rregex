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
    Class(Vec<ClassType>, bool), // bool indicates negation
    Any,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClassType {
    Range(char, char),
    Single(char),
}
