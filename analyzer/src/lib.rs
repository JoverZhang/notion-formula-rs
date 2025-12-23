use crate::{
    ast::Expr,
    lexer::lex,
    parser::{ParseError, Parser},
};

mod ast;
mod lexer;
mod parser;
mod token;
mod tests;

pub fn analyze(text: &str) -> Result<Expr, ParseError> {
    let tokens = lex(text).unwrap();
    let mut parser = Parser::new(text, tokens);
    parser.parse_expr()
}
