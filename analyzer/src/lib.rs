use crate::{
    lexer::lex,
    parser::{ParseOutput, Parser},
    token::Span,
    tokenstream::TokenCursor,
};

mod ast;
mod diagnostics;
mod lexer;
mod parser;
mod source_map;
mod tests;
mod token;
mod tokenstream;

pub fn analyze(text: &str) -> Result<ParseOutput, diagnostics::Diagnostic> {
    let tokens = lex(&text).map_err(|msg| diagnostics::Diagnostic {
        kind: diagnostics::DiagnosticKind::Error,
        message: msg,
        span: Span { start: 0, end: 0 },
        labels: vec![],
        notes: vec![],
    })?;
    let token_cursor = TokenCursor::new(&text, tokens);
    let mut parser = Parser::new(token_cursor);
    Ok(parser.parse_expr())
}

pub use diagnostics::format_diagnostics;
pub use diagnostics::{Diagnostic, DiagnosticKind, Diagnostics};
