use crate::parser::ast::Ast;

#[derive(Debug, PartialEq, Eq)]
pub struct BoundedRepetition {
    pub ast: Box<Ast>,
    pub n: u16,
    pub m: Option<u16>,
    pub lazy: bool,
}
