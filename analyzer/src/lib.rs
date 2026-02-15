//! Core formula analyzer.
//!
//! Pipeline: lex → parse → analyze/format → completion.
//! All spans are UTF-8 byte offsets into the original source, using `[start, end)`.
//! UTF-16 conversion for editors happens in `analyzer_wasm`.
use crate::{lexer::lex, parser::Parser};

pub mod analysis;
mod diagnostics;
mod lexer;
mod parser;
mod source_map;
mod span;
mod tests;
mod text_edit;

pub use parser::ParseOutput;
pub type SyntaxResult = ParseOutput;

#[derive(Debug, Clone)]
pub struct AnalyzeResult {
    pub diagnostics: Vec<diagnostics::Diagnostic>,
    pub tokens: Vec<lexer::Token>,
    pub output_type: analysis::Ty,
}

pub fn analyze_syntax(text: &str) -> SyntaxResult {
    let lex_output = lex(text);
    let token_cursor = parser::TokenCursor::new(text, lex_output.tokens);
    let mut parser = Parser::new(token_cursor);
    let mut output = parser.parse();
    output.diagnostics.extend(lex_output.diagnostics);
    output
}

pub fn analyze(text: &str, ctx: &analysis::Context) -> AnalyzeResult {
    let mut syntax = analyze_syntax(text);
    let (output_type, sema_diags) = analysis::analyze_expr(&syntax.expr, ctx);
    syntax.diagnostics.extend(sema_diags);

    AnalyzeResult {
        diagnostics: syntax.diagnostics,
        tokens: syntax.tokens,
        output_type,
    }
}

pub use analysis as semantic;
pub use analysis::{ExprId, TypeMap, infer_expr_with_map};
pub use diagnostics::format_diagnostics;
pub use diagnostics::{
    CodeAction, Diagnostic, DiagnosticCode, DiagnosticKind, Diagnostics, ParseDiagnostic,
};
pub use lexer::Lit;
pub use lexer::{CommentKind, LitKind, Token, TokenKind};
pub use lexer::{NodeId, Symbol, TokenIdx, TokenRange, tokens_in_span};
pub use parser::TokenQuery;
pub use parser::ast;
pub use source_map::SourceMap;
pub use span::{Span, Spanned};
pub use text_edit::TextEdit;
