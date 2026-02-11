use analyzer::{Diagnostic, DiagnosticKind, SourceMap, Span, Token, TokenKind};

use crate::dto::v1::{
    CodeActionView, DiagnosticKindView, DiagnosticView, Span as SpanDto, SpanView, TextEditView,
    TokenView,
};
use crate::span::byte_span_to_utf16_span;

pub(crate) fn diagnostic_view(
    source: &str,
    sm: &SourceMap<'_>,
    diag: &Diagnostic,
) -> DiagnosticView {
    let (line, col) = sm.line_col(diag.span.start);

    DiagnosticView {
        kind: diagnostic_kind_view(&diag.kind),
        message: diag.message.clone(),
        span: span_view(source, diag.span),
        line,
        col,
        actions: diag
            .actions
            .iter()
            .map(|action| CodeActionView {
                title: action.title.clone(),
                edits: action
                    .edits
                    .iter()
                    .map(|edit| TextEditView {
                        range: span_dto(source, edit.range),
                        new_text: edit.new_text.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub(crate) fn token_view(source: &str, token: &Token) -> TokenView {
    let start = token.span.start as usize;
    let end = token.span.end as usize;
    let text = source.get(start..end).unwrap_or("").to_string();

    TokenView {
        kind: token_kind_string(&token.kind).to_string(),
        text,
        span: span_view(source, token.span),
    }
}

pub(crate) fn span_view(source: &str, span: Span) -> SpanView {
    let range = byte_span_to_utf16_span(source, span);
    SpanView { range }
}

pub(crate) fn span_dto(source: &str, span: Span) -> SpanDto {
    byte_span_to_utf16_span(source, span)
}

fn diagnostic_kind_view(kind: &DiagnosticKind) -> DiagnosticKindView {
    match kind {
        DiagnosticKind::Error => DiagnosticKindView::Error,
    }
}

fn token_kind_string(kind: &TokenKind) -> &'static str {
    use TokenKind::*;
    use analyzer::LitKind;

    match kind {
        Lt => "Lt",
        Le => "Le",
        EqEq => "EqEq",
        Ne => "Ne",
        Ge => "Ge",
        Gt => "Gt",
        AndAnd => "AndAnd",
        OrOr => "OrOr",
        Bang => "Bang",
        Not => "Not",
        Plus => "Plus",
        Minus => "Minus",
        Star => "Star",
        Slash => "Slash",
        Percent => "Percent",
        Caret => "Caret",
        Dot => "Dot",
        Comma => "Comma",
        Colon => "Colon",
        Pound => "Pound",
        Question => "Question",
        OpenParen => "OpenParen",
        CloseParen => "CloseParen",
        OpenBracket => "OpenBracket",
        CloseBracket => "CloseBracket",
        Literal(lit) => match lit.kind {
            LitKind::Bool => "Bool",
            LitKind::Number => "Number",
            LitKind::String => "String",
        },
        Ident(_) => "Ident",
        DocComment(kind, _) => match kind {
            analyzer::CommentKind::Line => "LineComment",
            analyzer::CommentKind::Block => "BlockComment",
        },
        Newline => "Newline",
        Eof => "Eof",
    }
}
