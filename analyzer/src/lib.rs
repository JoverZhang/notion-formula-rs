use crate::{
    ast::Expr,
    lexer::lex,
    parser::{ParseError, Parser},
    tokenstream::TokenCursor,
};

mod ast;
mod lexer;
mod parser;
mod tests;
mod token;
mod tokenstream;

pub fn analyze(text: &str) -> Result<Expr, ParseError> {
    let tokens = lex(&text).unwrap();
    let token_cursor = TokenCursor::new(&text, tokens);
    let mut parser = Parser::new(token_cursor);
    parser.parse_expr()
}
