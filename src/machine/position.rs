#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Position {
    Start,
    End,
}

impl From<&crate::parser::ast::AnchorKind> for Position {
    fn from(anchor: &crate::parser::ast::AnchorKind) -> Self {
        match anchor {
            crate::parser::ast::AnchorKind::Start => Position::Start,
            crate::parser::ast::AnchorKind::End => Position::End,
        }
    }
}
