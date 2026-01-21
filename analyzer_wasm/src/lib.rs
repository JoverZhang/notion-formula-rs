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

#[derive(Serialize)]
struct SimpleSpanView {
    start: usize,
    end: usize,
}

#[derive(Serialize)]
struct TextEditView {
    range: SimpleSpanView,
    new_text: String,
}

#[derive(Serialize)]
struct SignatureHelpView {
    label: String,
    params: Vec<String>,
    active_param: usize,
}

#[derive(Serialize)]
struct CompletionItemView {
    label: String,
    kind: String,
    insert_text: String,
    primary_edit: Option<TextEditView>,
    cursor: Option<usize>,
    additional_edits: Vec<TextEditView>,
    detail: Option<String>,
    is_disabled: bool,
    disabled_reason: Option<String>,
}

#[derive(Serialize)]
struct CompletionOutputView {
    items: Vec<CompletionItemView>,
    replace: SimpleSpanView,
    signature_help: Option<SignatureHelpView>,
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

#[wasm_bindgen]
pub fn complete(source: String, cursor_utf16: usize, context_json: Option<String>) -> JsValue {
    let cursor_byte = utf16_offset_to_byte(&source, cursor_utf16);
    let ctx = parse_context(context_json.as_deref());

    let output = analyzer::complete_with_context(&source, cursor_byte, ctx.as_ref());
    let view = completion_output_to_view(&source, &output);
    serde_wasm_bindgen::to_value(&view).unwrap_or(JsValue::NULL)
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

fn parse_context(context_json: Option<&str>) -> Option<Context> {
    match context_json.map(str::trim) {
        None | Some("") => None,
        Some(json) => serde_json::from_str::<Context>(json).ok(),
    }
}

fn utf16_offset_to_byte(source: &str, utf16: usize) -> usize {
    if utf16 == 0 {
        return 0;
    }
    let mut u16_count = 0usize;
    for (byte_idx, ch) in source.char_indices() {
        if u16_count >= utf16 {
            return byte_idx;
        }
        u16_count += ch.len_utf16();
    }
    source.len()
}

fn simple_span_view(source: &str, span: Span) -> SimpleSpanView {
    SimpleSpanView {
        start: byte_offset_to_utf16(source, span.start as usize),
        end: byte_offset_to_utf16(source, span.end as usize),
    }
}

fn completion_kind_string(kind: analyzer::CompletionKind) -> &'static str {
    use analyzer::CompletionKind::*;
    match kind {
        Function => "Function",
        Builtin => "Builtin",
        Property => "Property",
        Operator => "Operator",
    }
}

fn apply_text_edits_bytes(source: &str, edits: &[analyzer::TextEdit]) -> String {
    let mut sorted = edits.to_vec();
    sorted.sort_by(|a, b| {
        b.range
            .start
            .cmp(&a.range.start)
            .then(b.range.end.cmp(&a.range.end))
    });

    let mut updated = source.to_string();
    for edit in sorted {
        let start = edit.range.start as usize;
        let end = edit.range.end as usize;
        if start > end || end > updated.len() {
            continue;
        }
        if !updated.is_char_boundary(start) || !updated.is_char_boundary(end) {
            continue;
        }

        let mut next = String::with_capacity(updated.len() - (end - start) + edit.new_text.len());
        next.push_str(&updated[..start]);
        next.push_str(&edit.new_text);
        next.push_str(&updated[end..]);
        updated = next;
    }
    updated
}

fn completion_output_to_view(
    source: &str,
    output: &analyzer::CompletionOutput,
) -> CompletionOutputView {
    let replace = simple_span_view(source, output.replace);
    let signature_help = output.signature_help.as_ref().map(|sig| SignatureHelpView {
        label: sig.label.clone(),
        params: sig.params.clone(),
        active_param: sig.active_param,
    });

    let items = output
        .items
        .iter()
        .map(|item| completion_item_to_view(source, output, item))
        .collect();

    CompletionOutputView {
        items,
        replace,
        signature_help,
    }
}

fn completion_item_to_view(
    source: &str,
    output: &analyzer::CompletionOutput,
    item: &analyzer::CompletionItem,
) -> CompletionItemView {
    let primary_edit_view = item.primary_edit.as_ref().map(|edit| TextEditView {
        range: simple_span_view(source, edit.range),
        new_text: edit.new_text.clone(),
    });
    let additional_edits = item
        .additional_edits
        .iter()
        .map(|edit| TextEditView {
            range: simple_span_view(source, edit.range),
            new_text: edit.new_text.clone(),
        })
        .collect::<Vec<_>>();

    let cursor_utf16 = item.primary_edit.as_ref().map(|primary_edit| {
        let mut edits = Vec::with_capacity(1 + item.additional_edits.len());
        edits.push(primary_edit.clone());
        edits.extend(item.additional_edits.iter().cloned());
        let updated = apply_text_edits_bytes(source, &edits);

        let cursor_byte = item.cursor.unwrap_or_else(|| {
            output
                .replace
                .start
                .saturating_add(primary_edit.new_text.len() as u32)
        });
        let cursor_byte = usize::min(cursor_byte as usize, updated.len());
        byte_offset_to_utf16(&updated, cursor_byte)
    });

    CompletionItemView {
        label: item.label.clone(),
        kind: completion_kind_string(item.kind).to_string(),
        insert_text: item.insert_text.clone(),
        primary_edit: primary_edit_view,
        cursor: cursor_utf16,
        additional_edits,
        detail: item.detail.clone(),
        is_disabled: item.is_disabled,
        disabled_reason: item.disabled_reason.clone(),
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
