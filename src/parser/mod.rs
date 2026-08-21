use crate::parser::cursor::Cursor;

mod cursor;

pub enum ParserError {
    UnexpectedToken(char),
    UnexpectedEndOfInput,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Ast {
    Empty,
    Literal(char),
    Concat(Vec<Ast>),
    Alternation(Vec<Ast>),
    Star(Box<Ast>),
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
    let mut nodes: Vec<Ast> = vec![];

    let node = parse_concat(cursor)?;
    nodes.push(node);

    while cursor.eat('|') {
        let node = parse_concat(cursor)?;
        nodes.push(node);
    }
    match nodes.len() {
        0 => Ok(Ast::Empty),
        1 => return Ok(nodes.pop().unwrap()),
        _ => return Ok(Ast::Alternation(nodes)),
    }
}

fn parse_concat(cursor: &mut Cursor) -> Result<Ast, ParserError> {
    let mut nodes: Vec<Ast> = vec![];

    loop {
        let next = cursor.peek();
        let c = match next {
            Some(c) => c,
            None => break,
        };
        if c == ')' || c == '|' {
            break;
        }

        let node = parse_star(cursor)?;
        nodes.push(node);
    }
    match nodes.len() {
        0 => Ok(Ast::Empty),
        1 => return Ok(nodes.pop().unwrap()),
        _ => return Ok(Ast::Concat(nodes)),
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
