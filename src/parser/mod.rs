use crate::{
    cursor::{Cursor, OverFlowResult},
    parser::{
        ast::{AnchorKind, Ast, ClassSet, ClassType},
        bounded_repetition::BoundedRepetition,
    },
};

pub mod ast;
pub mod bounded_repetition;

#[derive(Debug, PartialEq, Eq)]
pub enum ParserError {
    UnexpectedToken(char),
    InvalidRange(char, char),
    LimitExceeded(String),
    MinimumGreaterThanMaximum(u16, u16),
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
            '{' => {
                let initial_offset: usize = cursor.offset();
                node = parse_bonded_repetition(cursor, node)?;
                if cursor.offset() == initial_offset {
                    //we didn't consume any tokens, so we should break to avoid an infinite loop
                    break;
                }
            }
            _ => break,
        }
    }
    Ok(node)
}

fn parse_bonded_repetition(cursor: &mut Cursor, node: Ast) -> Result<Ast, ParserError> {
    //We are inside a peek so we need to keep track of our offset
    let mut offset = 1;
    offset = cursor.offset_whitespace_at(offset);
    let n1 = match cursor.peek_at(offset) {
        None => return Ok(node),
        Some(c) => c,
    };
    if n1 == ',' {
        return parse_bonded_repetition_comma_start(cursor, node, offset);
    }
    let n1 = match cursor.peek_number_at(offset) {
        None => return Ok(node),
        Some(r) => r,
    };
    offset = n1.offset;
    offset = cursor.offset_whitespace_at(offset);
    let n2 = match cursor.peek_at(offset) {
        None => return Ok(node),
        Some(c) => c,
    };
    if n2 != ',' && n2 != '}' {
        return Ok(node);
    }
    offset += 1;
    if n2 == '}' {
        return parse_bonded_repetition_exact(cursor, node, n1, offset);
    }
    offset = cursor.offset_whitespace_at(offset);
    if cursor.peek_at(offset) == Some('}') {
        offset += 1;
        return parse_bonded_repetition_unbounded(cursor, node, n1, offset);
    }
    let n3 = match cursor.peek_number_at(offset) {
        None => return Ok(node),
        Some(r) => r,
    };
    offset = n3.offset;
    offset = cursor.offset_whitespace_at(offset);
    let n4 = match cursor.peek_at(offset) {
        Some(c) => c,
        None => return Ok(node),
    };
    if n4 != '}' {
        return Ok(node);
    }
    offset += 1;
    let (n, m) = match (n1.result, n3.result) {
        (Err(_), _) | (_, Err(_)) => {
            let string = cursor.string_offset(offset);
            return Err(ParserError::LimitExceeded(string));
        }
        (Ok(n), Ok(m)) => (n, m),
    };
    if n > m {
        return Err(ParserError::MinimumGreaterThanMaximum(n, m));
    }
    let lazy = check_lazy(cursor, &mut offset);
    cursor.move_to(offset);
    let bonded_repetition = BoundedRepetition {
        ast: Box::new(node),
        lazy,
        n,
        m: Some(m),
    };
    return Ok(Ast::BoundedRepetition(bonded_repetition));
}

fn parse_bonded_repetition_comma_start(
    cursor: &mut Cursor,
    node: Ast,
    mut offset: usize,
) -> Result<Ast, ParserError> {
    offset += 1;
    offset = cursor.offset_whitespace_at(offset);
    let n2 = match cursor.peek_number_at(offset) {
        None => return Ok(node),
        Some(r) => r,
    };

    let num = match n2.result {
        Err(_) => {
            offset = n2.offset;
            offset = cursor.offset_whitespace_at(offset);
            if cursor.peek_at(offset) == Some('}') {
                offset += 1;
                let string = cursor.string_offset(offset);
                return Err(ParserError::LimitExceeded(string));
            }
            return Ok(node);
        }
        Ok(r) => r,
    };

    offset = n2.offset;
    offset = cursor.offset_whitespace_at(offset);
    if cursor.peek_at(offset) != Some('}') {
        return Ok(node);
    }
    offset += 1;
    let lazy = check_lazy(cursor, &mut offset);
    cursor.move_to(offset);
    let bonded_repetition = BoundedRepetition {
        ast: Box::new(node),
        lazy,
        n: 0,
        m: Some(num),
    };
    return Ok(Ast::BoundedRepetition(bonded_repetition));
}

fn parse_bonded_repetition_exact(
    cursor: &mut Cursor,
    node: Ast,
    result: OverFlowResult,
    mut offset: usize,
) -> Result<Ast, ParserError> {
    let num = match result.result {
        Err(_) => {
            let string = cursor.string_offset(offset);
            return Err(ParserError::LimitExceeded(string));
        }
        Ok(r) => r,
    };

    let lazy = check_lazy(cursor, &mut offset);
    cursor.move_to(offset);
    let bonded_repetition = BoundedRepetition {
        ast: Box::new(node),
        lazy,
        n: num,
        m: Some(num),
    };
    return Ok(Ast::BoundedRepetition(bonded_repetition));
}

fn parse_bonded_repetition_unbounded(
    cursor: &mut Cursor,
    node: Ast,
    result: OverFlowResult,
    mut offset: usize,
) -> Result<Ast, ParserError> {
    let num = match result.result {
        Err(_) => {
            let string = cursor.string_offset(offset);
            return Err(ParserError::LimitExceeded(string));
        }
        Ok(r) => r,
    };

    let lazy = check_lazy(cursor, &mut offset);
    cursor.move_to(offset);
    let bonded_repetition = BoundedRepetition {
        ast: Box::new(node),
        lazy,
        n: num,
        m: None,
    };
    return Ok(Ast::BoundedRepetition(bonded_repetition));
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
        Some('^') => {
            cursor.next();
            Ok(Ast::Anchor(AnchorKind::Start))
        }
        Some('$') => {
            cursor.next();
            Ok(Ast::Anchor(AnchorKind::End))
        }
        Some(c) => {
            cursor.next();
            Ok(Ast::Literal(c))
        }
    }
}

fn parse_class(cursor: &mut Cursor) -> Result<Ast, ParserError> {
    let mut class = ClassSet::new();
    let mut negation = false;
    let mut start = true;
    loop {
        let peek = cursor.peek();
        match peek {
            None => return Err(ParserError::UnexpectedEndOfInput),
            Some(']') => return Ok(Ast::Class(class, negation)),
            Some(c) => {
                cursor.next();
                if c == '^' && start {
                    negation = true;
                    start = false;
                    continue;
                }
                start = false;

                let n = match cursor.peek() {
                    None | Some(']') => {
                        class.push(ClassType::Single(c));
                        continue;
                    }
                    Some(n) => n,
                };

                if n != '-' {
                    class.push(ClassType::Single(c));
                    continue;
                }

                let n2 = match cursor.peek_at(1) {
                    None | Some(']') => {
                        class.push(ClassType::Single(c));

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
                class.push(ClassType::Range(c, n2));
            }
        }
    }
}

fn check_lazy(cursor: &Cursor, offset: &mut usize) -> bool {
    if cursor.peek_at(*offset) == Some('?') {
        *offset += 1;
        return true;
    }
    return false;
}
#[cfg(test)]
mod tests;
