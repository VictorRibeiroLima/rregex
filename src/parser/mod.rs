use crate::{
    cursor::Cursor,
    parser::ast::{Ast, ClassType},
};

pub mod ast;

#[derive(Debug, PartialEq, Eq)]
pub enum ParserError {
    UnexpectedToken(char),
    InvalidRange(char, char),
    UnexpectedEndOfInput,
}

pub fn parse(input: &str) -> Result<Ast, ParserError> {
    let mut parser = Cursor::new(input);
    let result = parse_alternation(&mut parser)?;
    if parser.is_eof() {
        Ok(result)
    } else {
        Err(ParserError::UnexpectedToken(parser.peek().unwrap()))
    }
}

fn parse_alternation(cursor: &mut Cursor) -> Result<Ast, ParserError> {
    let node1 = parse_concat(cursor)?;
    if !cursor.eat('|') {
        return Ok(node1);
    }
    let node2 = parse_alternation(cursor)?;
    return Ok(Ast::Alternation(Box::new(node1), Box::new(node2)));
}

fn parse_concat(cursor: &mut Cursor) -> Result<Ast, ParserError> {
    let peak = cursor.peek();
    match peak {
        None => return Ok(Ast::Empty),
        Some(')') | Some('|') => return Ok(Ast::Empty),
        _ => {}
    }
    let node1 = parse_repetition(cursor)?;

    let peak = cursor.peek();
    let node2 = match peak {
        None => Ast::Empty,
        Some(')') | Some('|') => Ast::Empty,
        _ => parse_concat(cursor)?,
    };

    match node2 {
        Ast::Empty => Ok(node1),
        _ => Ok(Ast::Concat(Box::new(node1), Box::new(node2))),
    }
}

fn parse_repetition(cursor: &mut Cursor) -> Result<Ast, ParserError> {
    let mut node = parse_atom(cursor)?;
    while let Some(c) = cursor.peek() {
        match c {
            '*' => {
                cursor.next();
                if cursor.peek() == Some('?') {
                    cursor.next();
                    node = Ast::LazyStar(Box::new(node));
                    continue;
                }
                node = Ast::Star(Box::new(node));
            }
            '+' => {
                cursor.next();
                if cursor.peek() == Some('?') {
                    cursor.next();
                    node = Ast::LazyPlus(Box::new(node));
                    continue;
                }
                node = Ast::Plus(Box::new(node));
            }
            '?' => {
                cursor.next();
                if cursor.peek() == Some('?') {
                    cursor.next();
                    node = Ast::LazyQuestion(Box::new(node));
                    continue;
                }
                node = Ast::Question(Box::new(node));
            }
            _ => break,
        }
    }
    Ok(node)
}

fn parse_atom(cursor: &mut Cursor) -> Result<Ast, ParserError> {
    let peek = cursor.peek();
    match peek {
        None => Err(ParserError::UnexpectedEndOfInput),
        Some('.') => {
            cursor.next();
            Ok(Ast::Any)
        }
        Some('[') => {
            cursor.next();
            let node = parse_class(cursor)?;
            if cursor.peek() == Some(']') {
                cursor.next();
                return Ok(node);
            } else {
                return Err(ParserError::UnexpectedToken(cursor.peek().unwrap_or('\0')));
            }
        }
        Some('*' | '+' | '?') => Err(ParserError::UnexpectedToken(peek.unwrap())),
        Some('|') | Some(')') => Err(ParserError::UnexpectedToken(peek.unwrap())),
        Some('(') => {
            cursor.next();
            let node = parse_alternation(cursor)?;
            if cursor.peek() == Some(')') {
                cursor.next();
                return Ok(node);
            } else {
                return Err(ParserError::UnexpectedToken(cursor.peek().unwrap_or('\0')));
            }
        }
        Some('\\') => {
            cursor.next();
            let escaped = cursor.next();
            match escaped {
                None => Err(ParserError::UnexpectedEndOfInput),
                Some(c) => Ok(Ast::Literal(c)),
            }
        }
        Some(c) => {
            cursor.next();
            Ok(Ast::Literal(c))
        }
    }
}

fn parse_class(cursor: &mut Cursor) -> Result<Ast, ParserError> {
    let mut classes = vec![]; // Placeholder for actual class parsing logic]
    let mut negation = false;
    let mut start = true;
    loop {
        let peek = cursor.peek();
        match peek {
            None => return Err(ParserError::UnexpectedEndOfInput),
            Some(']') => return Ok(Ast::Class(classes, negation)),
            Some(c) => {
                cursor.next();
                if c == '^' && start {
                    negation = true;
                    start = false;
                    continue;
                }

                let n = match cursor.peek() {
                    None | Some(']') => {
                        classes.push(ClassType::Single(c));
                        start = false;
                        continue;
                    }
                    Some(n) => n,
                };

                if n != '-' {
                    classes.push(ClassType::Single(c));
                    start = false;
                    continue;
                }

                let n2 = match cursor.peek_at(1) {
                    None | Some(']') => {
                        classes.push(ClassType::Single(c));
                        start = false;
                        continue;
                    }
                    Some(n) => n,
                };
                //We are at a range consume the tokens
                cursor.next();
                cursor.next();

                if c > n2 {
                    return Err(ParserError::InvalidRange(c, n2));
                }
                classes.push(ClassType::Range(c, n2));
            }
        }
        start = false;
    }
}

#[cfg(test)]
mod tests;
