use analyzer::semantic::Context;
use analyzer::{
    Diagnostic, DiagnosticKind, ParseOutput, SourceMap, Span, Token, TokenKind,
    byte_offset_to_utf16,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct SpanView {
    start: usize,
    end: usize,
    line: usize,
    col: usize,
}

#[derive(Serialize)]
struct DiagnosticView {
    kind: String,
    message: String,
    span: SpanView,
}

#[derive(Serialize)]
struct TokenView {
    kind: String,
    text: String,
    span: SpanView,
}

#[derive(Serialize)]
struct AnalyzeResult {
    diagnostics: Vec<DiagnosticView>,
    tokens: Vec<TokenView>,
    formatted: String,
}

#[wasm_bindgen]
pub fn analyze(source: String, context_json: Option<String>) -> JsValue {
    let result = match context_json.as_deref().map(str::trim) {
        None | Some("") => match analyzer::analyze(&source) {
            Ok(output) => analyze_output(&source, output),
            Err(diag) => AnalyzeResult {
                diagnostics: vec![diag_to_view(&source, &diag)],
                tokens: Vec::new(),
                formatted: String::new(),
            },
        },
        Some(context_json) => match serde_json::from_str::<Context>(context_json) {
            Ok(ctx) => match analyzer::analyze_with_context(&source, ctx) {
                Ok(output) => analyze_output(&source, output),
                Err(diag) => AnalyzeResult {
                    diagnostics: vec![diag_to_view(&source, &diag)],
                    tokens: Vec::new(),
                    formatted: String::new(),
                },
            },
            Err(_) => {
                let mut result = match analyzer::analyze(&source) {
                    Ok(output) => analyze_output(&source, output),
                    Err(diag) => AnalyzeResult {
                        diagnostics: vec![diag_to_view(&source, &diag)],
                        tokens: Vec::new(),
                        formatted: String::new(),
                    },
                };

                let sm = SourceMap::new(&source);
                result
                    .diagnostics
                    .push(invalid_context_diag_view(&source, &sm));
                result
            }
        },
    };

    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

fn analyze_output(source: &str, output: ParseOutput) -> AnalyzeResult {
    let sm = SourceMap::new(source);
    let diagnostics = output
        .diagnostics
        .iter()
        .map(|diag| diag_to_view_with_sm(source, &sm, diag))
        .collect();

    let tokens = output
        .tokens
        .iter()
        .filter(|token| !token.is_trivia())
        .map(|token| token_to_view(source, &sm, token))
        .collect();

    let formatted = analyzer::format_expr(&output.expr, source, &output.tokens);

    AnalyzeResult {
        diagnostics,
        tokens,
        formatted,
    }
}

fn diag_to_view(source: &str, diag: &Diagnostic) -> DiagnosticView {
    let sm = SourceMap::new(source);
    diag_to_view_with_sm(source, &sm, diag)
}

fn diag_to_view_with_sm(source: &str, sm: &SourceMap, diag: &Diagnostic) -> DiagnosticView {
    DiagnosticView {
        kind: diagnostic_kind_string(&diag.kind).to_string(),
        message: diag.message.clone(),
        span: span_view(source, sm, diag.span),
    }
}

fn invalid_context_diag_view(source: &str, sm: &SourceMap) -> DiagnosticView {
    let diag = Diagnostic {
        kind: DiagnosticKind::Error,
        message: "Invalid context JSON".into(),
        span: Span { start: 0, end: 0 },
        labels: vec![],
        notes: vec![],
    };
    diag_to_view_with_sm(source, sm, &diag)
}

fn token_to_view(source: &str, sm: &SourceMap, token: &Token) -> TokenView {
    let start = token.span.start as usize;
    let end = token.span.end as usize;
    let text = source.get(start..end).unwrap_or("").to_string();

    TokenView {
        kind: token_kind_string(&token.kind).to_string(),
        text,
        span: span_view(source, sm, token.span),
    }
}

fn span_view(source: &str, sm: &SourceMap, span: Span) -> SpanView {
    let start = byte_offset_to_utf16(source, span.start as usize);
    let end = byte_offset_to_utf16(source, span.end as usize);
    let (line, col) = sm.line_col(span.start);

    SpanView {
        start,
        end,
        line,
        col,
    }
}

fn diagnostic_kind_string(kind: &DiagnosticKind) -> &'static str {
    match kind {
        DiagnosticKind::Error => "error",
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
        Literal(lit) => match lit.kind {
            LitKind::Bool => "Bool",
            LitKind::Number => "Number",
            LitKind::String => "String",
        },
        Ident(_) => "Ident",
        DocComment(..) => "DocComment",
        LineComment(_) => "LineComment",
        BlockComment(_) => "BlockComment",
        Newline => "Newline",
        Eof => "Eof",
    }
}
