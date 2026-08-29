use crate::cursor::Cursor;

#[derive(Debug, PartialEq, Eq)]
pub enum ParserError {
    UnexpectedToken(char),
    UnexpectedEndOfInput,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Ast {
    Empty,
    Literal(char),
    Concat(Box<Ast>, Box<Ast>),
    Alternation(Box<Ast>, Box<Ast>),
    Star(Box<Ast>),
    Any,
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
    let node1 = parse_star(cursor)?;

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

fn parse_star(cursor: &mut Cursor) -> Result<Ast, ParserError> {
    let mut node = parse_atom(cursor)?;
    //A repetition can wrap another repetition
    while let Some('*') = cursor.peek() {
        cursor.next();
        node = Ast::Star(Box::new(node));
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
            todo!("escape sequences");
        }
        Some(c) => {
            cursor.next();
            Ok(Ast::Literal(c))
        }
    }
}

#[cfg(test)]
mod tests;
