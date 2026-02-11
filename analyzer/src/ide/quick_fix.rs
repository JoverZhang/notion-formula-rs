//! Structured quick-fix helpers for editor integrations.
//! Coordinates are UTF-8 byte offsets with half-open ranges `[start, end)`.

use std::collections::HashSet;

use crate::ast::Expr;
use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ide::format::format_expr;
use crate::lexer::{Span, Token};

/// A single byte-range edit for a quick fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickFixEdit {
    pub range: Span,
    pub new_text: String,
}

/// A single quick fix action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickFix {
    pub title: String,
    pub edits: Vec<QuickFixEdit>,
}

/// Returns true when diagnostics include lexing/parsing failures.
pub fn has_syntax_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|d| matches!(d.code, DiagnosticCode::LexError | DiagnosticCode::Parse(_)))
}

/// Collect structured quick fixes from diagnostic labels.
pub fn quick_fixes(diagnostics: &[Diagnostic]) -> Vec<QuickFix> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for diag in diagnostics {
        for label in &diag.labels {
            let Some(fix) = &label.quick_fix else {
                continue;
            };

            if !seen.insert((label.span.start, label.span.end, fix.new_text.clone())) {
                continue;
            }

            out.push(QuickFix {
                title: fix.title.clone(),
                edits: vec![QuickFixEdit {
                    range: label.span,
                    new_text: fix.new_text.clone(),
                }],
            });
        }
    }

    out
}

/// Returns canonical formatter output only when syntax is valid.
pub fn formatted_if_syntax_valid(
    expr: &Expr,
    source: &str,
    tokens: &[Token],
    diagnostics: &[Diagnostic],
) -> String {
    if has_syntax_errors(diagnostics) {
        return String::new();
    }
    format_expr(expr, source, tokens)
}
