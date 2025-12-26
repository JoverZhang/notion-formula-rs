use crate::source_map::SourceMap;
use crate::token::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub span: Span,
    pub notes: Vec<String>,
}

#[derive(Default, Debug)]
pub struct Diagnostics {
    pub diags: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn emit_error(&mut self, span: Span, message: impl Into<String>) {
        self.diags.push(Diagnostic {
            kind: DiagnosticKind::Error,
            message: message.into(),
            span,
            notes: vec![],
        });
    }
}

pub fn format_diagnostics(source: &str, mut diags: Vec<Diagnostic>) -> String {
    use std::fmt::Write;

    diags.sort_by_key(|d| (d.span.start, d.message.clone()));
    let sm = SourceMap::new(source);

    let mut out = String::new();
    for d in diags {
        let (line, col) = sm.line_col(d.span.start);
        let _ = writeln!(&mut out, "error: {}", d.message);
        let _ = writeln!(
            &mut out,
            "  --> <input>:{}:{} [{}..{}]",
            line, col, d.span.start, d.span.end
        );
        for note in d.notes {
            let _ = writeln!(&mut out, "  note: {}", note);
        }
    }
    out
}
