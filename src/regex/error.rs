use crate::parser::ParserError;

#[derive(Debug, PartialEq, Eq)]
pub enum RegexError {
    ParseError(String),
}

impl From<ParserError> for RegexError {
    fn from(err: ParserError) -> Self {
        match err {
            ParserError::UnexpectedToken(c) => {
                RegexError::ParseError(format!("Unexpected token '{}' in regex pattern", c))
            }
            ParserError::UnexpectedEndOfInput => {
                RegexError::ParseError("Unexpected end of input in regex pattern".to_string())
            }
            ParserError::InvalidRange(c1, c2) => RegexError::ParseError(format!(
                "Invalid range '{}' - '{}' in regex pattern",
                c1, c2
            )),
        }
    }
}
