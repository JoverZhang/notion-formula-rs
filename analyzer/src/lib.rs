use crate::{lexer::lex, parser::Parser, tokenstream::TokenCursor};

mod ast;
pub mod completion;
mod diagnostics;
mod format;
mod lexer;
mod parser;
pub mod semantic;
mod source_map;
mod tests;
mod token;
mod tokenstream;

pub use parser::ParseOutput;

pub fn analyze(text: &str) -> Result<ParseOutput, diagnostics::Diagnostic> {
    let lex_output = lex(text);
    let token_cursor = TokenCursor::new(text, lex_output.tokens);
    let mut parser = Parser::new(token_cursor);
    let mut output = parser.parse_expr();
    output.diagnostics.extend(lex_output.diagnostics);
    Ok(output)
}

pub use completion::{
    CompletionData, CompletionItem, CompletionKind, CompletionOutput, SignatureHelp, TextEdit,
    complete, CompletionConfig,
};
pub use diagnostics::format_diagnostics;
pub use diagnostics::{Diagnostic, DiagnosticKind, Diagnostics};
pub use format::format_expr;
pub use source_map::SourceMap;
pub use token::{LitKind, Span, Token, TokenKind};
