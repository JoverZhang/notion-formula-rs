use crate::token::Token;

pub struct TokenCursor<'a> {
    pub source: &'a str,
    pub tokens: Vec<Token>,
    pub pos: usize,
}

impl<'a> TokenCursor<'a> {
    pub fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        TokenCursor {
            source,
            tokens,
            pos: 0,
        }
    }
}
