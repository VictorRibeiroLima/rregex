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
            ParserError::LimitExceeded(n1) => {
                RegexError::ParseError(format!("{} Quantifier exceeds the supported limit", n1,))
            }
            ParserError::MinimumGreaterThanMaximum(n1, n2) => RegexError::ParseError(format!(
                "Invalid quantifier range: minimum {} is greater than maximum {}",
                n1, n2
            )),
        }
    }
}
