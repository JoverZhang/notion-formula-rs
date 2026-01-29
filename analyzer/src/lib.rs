//! Formula analyzer core.
//!
//! This crate implements the core language pipeline and editor tooling for Notion-style formulas.
//! It is designed to be deterministic and to keep all source locations in a single, canonical
//! coordinate space.
//!
//! **Pipeline (high level)**
//! - Lexing: `lexer` produces a token stream + lex diagnostics.
//! - Parsing: `parser` consumes tokens into an AST + parse diagnostics.
//! - Analysis: `analysis` (`pub use analysis as semantic`) performs semantic checks and provides
//!   IDE-facing data (e.g. signature help).
//! - Formatting: `ide::format::format_expr` formats an AST back to text.
//! - Completion: `ide::completion` computes completion items and edits.
//!
//! **Source location invariant**
//! - Core spans are [`Span`] values: **UTF-8 byte offsets** into the original source string.
//! - Spans and token ranges are **half-open** `[start, end)` (inclusive start, exclusive end).
//! - When slicing source by spans, the source must be the same string that was lexed/parsed.
//!
//! **UTF-16 conversion**
//! This crate does *not* use UTF-16 offsets. The UTF-16 ↔ byte bridging required for JS/editor
//! integration happens in the `analyzer_wasm` crate at the WASM boundary.
//!
//! **Entry points**
//! - [`analyze`]: lex + parse (returns [`ParseOutput`]).
//! - [`semantic`]: re-export of the semantic analysis module.
//! - [`completion`]/[`complete`]: completion engine and helper.
//! - [`format_expr`]: formatter entry point.
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
