//! Core formula analyzer.
//!
//! Pipeline: lex → parse → analyze/format → completion.
//! All spans are UTF-8 byte offsets into the original source, using `[start, end)`.
//! UTF-16 conversion for editors happens in `analyzer_wasm`.
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
    let mut output = parser.parse();
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
pub use ide::quick_fix::{
    QuickFix, QuickFixEdit, formatted_if_syntax_valid, has_syntax_errors, quick_fixes,
};
pub use lexer::{CommentKind, LitKind, Span, Token, TokenKind};
pub use lexer::{NodeId, Spanned, Symbol, TokenIdx, TokenRange, tokens_in_span};
pub use parser::ast;
pub use source_map::SourceMap;
