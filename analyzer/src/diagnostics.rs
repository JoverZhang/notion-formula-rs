use crate::lexer::Span;
use crate::source_map::SourceMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub span: Span,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub message: Option<String>,
}

#[derive(Default, Debug)]
pub struct Diagnostics {
    pub diags: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn emit_error(&mut self, span: Span, message: impl Into<String>) {
        self.emit_error_with_labels(span, message, vec![]);
    }

    pub fn emit_error_with_labels(
        &mut self,
        span: Span,
        message: impl Into<String>,
        labels: Vec<Label>,
    ) {
        self.diags.push(Diagnostic {
            kind: DiagnosticKind::Error,
            message: message.into(),
            span,
            labels,
            notes: vec![],
        });
    }
}

pub fn format_diagnostics(source: &str, mut diags: Vec<Diagnostic>) -> String {
    use std::fmt::Write;

    diags.sort_by(|a, b| {
        (a.span.start, a.span.end, &a.message).cmp(&(b.span.start, b.span.end, &b.message))
    });
    let sm = SourceMap::new(source);

    let mut out = String::new();

    for d in diags {
        let mut labels = d.labels;
        labels.sort_by(|a, b| {
            (a.span.start, a.span.end, a.message.as_deref().unwrap_or("")).cmp(&(
                b.span.start,
                b.span.end,
                b.message.as_deref().unwrap_or(""),
            ))
        });

        let (line, col) = sm.line_col(d.span.start);
        let _ = writeln!(&mut out, "error: {}", d.message);
        let _ = writeln!(
            &mut out,
            "  --> <input>:{}:{} [{}..{}]",
            line, col, d.span.start, d.span.end
        );
        for label in labels {
            let (line, col) = sm.line_col(label.span.start);
            let _ = writeln!(
                &mut out,
                "  = label: {}:{} [{}..{}] {}",
                line,
                col,
                label.span.start,
                label.span.end,
                label.message.unwrap_or_default()
            );
        }
        for note in d.notes {
            let _ = writeln!(&mut out, "  note: {}", note);
        }
    }
    out
}
