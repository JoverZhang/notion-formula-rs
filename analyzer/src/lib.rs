use crate::{
    lexer::lex,
    parser::{ParseError, ParseOutput, Parser},
    tokenstream::TokenCursor,
};

mod ast;
mod lexer;
mod parser;
mod tests;
mod token;
mod tokenstream;

pub fn analyze(text: &str) -> Result<ParseOutput, ParseError> {
    let tokens = lex(&text).map_err(ParseError::LexError)?;
    let token_cursor = TokenCursor::new(&text, tokens);
    let mut parser = Parser::new(token_cursor);
    Ok(parser.parse_expr())
}
