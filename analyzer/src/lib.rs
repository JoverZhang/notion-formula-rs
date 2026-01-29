use crate::{lexer::lex, parser::Parser};

pub mod analysis;
mod diagnostics;
pub mod ide;
mod lexer;
mod parser;
mod source_map;
mod tests;

pub use parser::ParseOutput;

pub fn analyze(text: &str) -> Result<ParseOutput, diagnostics::Diagnostic> {
    let lex_output = lex(text);
    let token_cursor = parser::TokenCursor::new(text, lex_output.tokens);
    let mut parser = Parser::new(token_cursor);
    let mut output = parser.parse_expr();
    output.diagnostics.extend(lex_output.diagnostics);
    Ok(output)
}

pub use analysis as semantic;
pub use diagnostics::format_diagnostics;
pub use diagnostics::{Diagnostic, DiagnosticKind, Diagnostics};
pub use ide::completion;
pub use ide::completion::{
    CompletionConfig, CompletionData, CompletionItem, CompletionKind, CompletionOutput,
    SignatureHelp, TextEdit, complete,
};
pub use ide::format::format_expr;
pub use lexer::{LitKind, Span, Token, TokenKind};
pub use lexer::{NodeId, Spanned, Symbol, TokenIdx, TokenRange, tokens_in_span};
pub use parser::ast;
pub use source_map::SourceMap;
