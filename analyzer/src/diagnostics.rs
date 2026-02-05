use crate::lexer::Span;
use crate::source_map::SourceMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    LexError,
    SemanticError,
    Parse(ParseDiagnostic),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseDiagnostic {
    UnclosedDelimiter,
    MismatchedDelimiter,
    MissingComma,
    TrailingComma,
    MissingExpr,
    UnexpectedToken,
}

impl DiagnosticCode {
    pub fn priority(self) -> u8 {
        match self {
            DiagnosticCode::Parse(ParseDiagnostic::UnclosedDelimiter) => 100,
            DiagnosticCode::Parse(ParseDiagnostic::MismatchedDelimiter) => 95,
            DiagnosticCode::LexError => 90,
            DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken) => 80,
            DiagnosticCode::Parse(ParseDiagnostic::MissingExpr) => 70,
            DiagnosticCode::Parse(ParseDiagnostic::MissingComma) => 60,
            DiagnosticCode::Parse(ParseDiagnostic::TrailingComma) => 50,
            DiagnosticCode::SemanticError => 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub code: DiagnosticCode,
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
    pub fn emit(&mut self, code: DiagnosticCode, span: Span, message: impl Into<String>) {
        self.emit_with_labels(code, span, message, vec![]);
    }

    pub fn emit_with_labels(
        &mut self,
        code: DiagnosticCode,
        span: Span,
        message: impl Into<String>,
        labels: Vec<Label>,
    ) {
        let mut diag = Diagnostic {
            kind: DiagnosticKind::Error,
            code,
            message: message.into(),
            span,
            labels,
            notes: vec![],
        };

        dedup_labels(&mut diag.labels);
        self.push(diag);
    }

    fn push(&mut self, diag: Diagnostic) {
        let Some(existing_idx) = self.diags.iter().position(|d| d.span == diag.span) else {
            self.diags.push(diag);
            return;
        };

        let existing_priority = self.diags[existing_idx].code.priority();
        let incoming_priority = diag.code.priority();

        if incoming_priority > existing_priority {
            self.diags[existing_idx] = diag;
            return;
        }

        if incoming_priority == existing_priority
            && self.diags[existing_idx].message == diag.message
        {
            let existing = &mut self.diags[existing_idx];
            existing.labels.extend(diag.labels);
            existing.notes.extend(diag.notes);
            dedup_labels(&mut existing.labels);
            dedup_notes(&mut existing.notes);
        }
    }
}

pub fn format_diagnostics(source: &str, mut diags: Vec<Diagnostic>) -> String {
    use std::cmp::Reverse;
    use std::fmt::Write;

    diags.sort_by(|a, b| {
        (
            a.span.start,
            a.span.end,
            Reverse(a.code.priority()),
            &a.message,
        )
            .cmp(&(
                b.span.start,
                b.span.end,
                Reverse(b.code.priority()),
                &b.message,
            ))
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

fn dedup_labels(labels: &mut Vec<Label>) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    labels.retain(|l| {
        let key = (
            l.span.start,
            l.span.end,
            l.message.as_deref().unwrap_or("").to_owned(),
        );
        seen.insert(key)
    });
}

fn dedup_notes(notes: &mut Vec<String>) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    notes.retain(|n| seen.insert(n.clone()));
}
