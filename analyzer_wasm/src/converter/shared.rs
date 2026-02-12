use analyzer::{
    CodeAction as ByteCodeAction, Diagnostic as ByteDiagnostic,
    DiagnosticKind as ByteDiagnosticKind, SourceMap, Span as ByteSpan, Token as ByteToken,
    TokenKind,
};

use crate::dto::v1::{
    CodeAction, Diagnostic, DiagnosticKind, Span as Utf16Span, TextEdit as Utf16TextEdit, Token,
};
use crate::span::byte_span_to_utf16_span;

pub(crate) fn diagnostic_view(
    source: &str,
    sm: &SourceMap<'_>,
    diag: &ByteDiagnostic,
) -> Diagnostic {
    let (line, col) = sm.line_col(diag.span.start);

    Diagnostic {
        kind: diagnostic_kind_view(&diag.kind),
        message: diag.message.clone(),
        span: span_dto(source, diag.span),
        line,
        col,
        actions: diag
            .actions
            .iter()
            .map(|action| code_action(source, action))
            .collect(),
    }
}

pub(crate) fn token_view(source: &str, token: &ByteToken) -> Token {
    let start = token.span.start as usize;
    let end = token.span.end as usize;
    let text = source.get(start..end).unwrap_or("").to_string();

    Token {
        kind: token_kind_string(&token.kind).to_string(),
        text,
        span: span_dto(source, token.span),
    }
}

fn code_action(source: &str, action: &ByteCodeAction) -> CodeAction {
    CodeAction {
        title: action.title.clone(),
        edits: action
            .edits
            .iter()
            .map(|edit| Utf16TextEdit {
                range: span_dto(source, edit.range),
                new_text: edit.new_text.clone(),
            })
            .collect(),
    }
}

pub(crate) fn span_dto(source: &str, span: ByteSpan) -> Utf16Span {
    byte_span_to_utf16_span(source, span)
}

fn diagnostic_kind_view(kind: &ByteDiagnosticKind) -> DiagnosticKind {
    match kind {
        ByteDiagnosticKind::Error => DiagnosticKind::Error,
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
