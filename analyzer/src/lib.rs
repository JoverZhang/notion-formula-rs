use crate::{lexer::lex, parser::Parser, tokenstream::TokenCursor};

mod ast;
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

pub fn analyze_with_context(
    text: &str,
    ctx: semantic::Context,
) -> Result<ParseOutput, diagnostics::Diagnostic> {
    let mut output = analyze(text)?;
    let (_, diags) = semantic::analyze_expr(&output.expr, &ctx);
    output.diagnostics.extend(diags);
    Ok(output)
}

pub use diagnostics::format_diagnostics;
pub use diagnostics::{Diagnostic, DiagnosticKind, Diagnostics};
pub use format::format_expr;
pub use source_map::{SourceMap, byte_offset_to_utf16};
pub use token::{LitKind, Span, Token, TokenKind};
