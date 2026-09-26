use super::*;

mod bounded_repetition;
mod class_escapes;
mod classes;
mod empty_branches;
mod errors;
mod escapes;
mod lazy_quantifiers;
mod structure;

/// Parse, or fail the test with a readable message.
/// Written out instead of `.unwrap()` so `ParserError` doesn't need `Debug`.
fn ast(input: &str) -> Ast {
    match parse(input) {
        Ok(a) => a,
        Err(_) => panic!("expected `{input}` to parse, got an error"),
    }
}

fn lit(c: char) -> Ast {
    Ast::Literal(c)
}

fn cat(left: Ast, right: Ast) -> Ast {
    Ast::Concat(Box::new(left), Box::new(right))
}

fn alt(left: Ast, right: Ast) -> Ast {
    Ast::Alternation(Box::new(left), Box::new(right))
}

fn star(inner: Ast) -> Ast {
    Ast::Star(Box::new(inner))
}

fn plus(inner: Ast) -> Ast {
    Ast::Plus(Box::new(inner))
}

fn question(inner: Ast) -> Ast {
    Ast::Question(Box::new(inner))
}

fn lazy_star(inner: Ast) -> Ast {
    Ast::LazyStar(Box::new(inner))
}

fn lazy_plus(inner: Ast) -> Ast {
    Ast::LazyPlus(Box::new(inner))
}

fn lazy_question(inner: Ast) -> Ast {
    Ast::LazyQuestion(Box::new(inner))
}

fn br(inner: Ast, n: u16, m: Option<u16>, lazy: bool) -> Ast {
    Ast::BoundedRepetition(BoundedRepetition {
        ast: Box::new(inner),
        n,
        m,
        lazy,
    })
}
