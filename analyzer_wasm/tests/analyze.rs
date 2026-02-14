use js_sys::Reflect;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

use analyzer_wasm::dto::v1::AnalyzerConfig;

#[derive(Deserialize)]
struct AnalyzeResult {
    diagnostics: Vec<Diagnostic>,
    tokens: Vec<Token>,
    output_type: String,
}

#[derive(Deserialize)]
struct ApplyResult {
    source: String,
    cursor: u32,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
struct Span {
    start: u32,
    end: u32,
}

#[derive(Deserialize)]
struct Diagnostic {
    kind: String,
    #[allow(dead_code)]
    message: String,
    span: Span,
    line: usize,
    col: usize,
    actions: Vec<CodeAction>,
}

#[derive(Deserialize)]
struct CodeAction {
    title: String,
    edits: Vec<TextEdit>,
}

#[derive(Deserialize)]
struct Token {
    kind: String,
    text: String,
    span: Span,
}

#[derive(Deserialize, Serialize, Clone)]
struct TextEdit {
    range: Span,
    new_text: String,
}

#[derive(Deserialize)]
struct CompletionResult {
    preferred_indices: Vec<usize>,
}

#[derive(Deserialize)]
struct HelpResult {
    completion: CompletionResult,
}

fn analyzer(preferred_limit: Option<usize>) -> analyzer_wasm::Analyzer {
    let config = AnalyzerConfig {
        properties: Vec::new(),
        preferred_limit,
    };
    let config = serde_wasm_bindgen::to_value(&config).expect("expected analyzer config JsValue");
    analyzer_wasm::Analyzer::new(config).expect("expected Analyzer::new Ok")
}

fn analyze_value(source: &str) -> AnalyzeResult {
    let value = analyzer(None)
        .analyze(source.to_string())
        .expect("expected analyze() Ok");
    serde_wasm_bindgen::from_value(value).expect("expected AnalyzeResult")
}

fn format_value(source: &str, cursor_utf16: u32) -> ApplyResult {
    let value = analyzer(None)
        .ide_format(source.to_string(), cursor_utf16)
        .expect("expected ide_format() Ok");
    serde_wasm_bindgen::from_value(value).expect("expected ApplyResult")
}

fn edit(start: u32, end: u32, new_text: &str) -> TextEdit {
    TextEdit {
        range: Span { start, end },
        new_text: new_text.to_string(),
    }
}

fn apply_edits_value(source: &str, edits: &[TextEdit], cursor_utf16: u32) -> ApplyResult {
    let edits: JsValue = serde_wasm_bindgen::to_value(edits).expect("expected edits JsValue");
    let value = analyzer(None)
        .ide_apply_edits(source.to_string(), edits, cursor_utf16)
        .expect("expected ide_apply_edits() Ok");
    serde_wasm_bindgen::from_value(value).expect("expected ApplyResult")
}

fn utf16_slice(source: &str, start: usize, end: usize) -> String {
    let mut utf16_pos = 0usize;
    let mut start_byte = None;
    let mut end_byte = None;

    for (byte_idx, ch) in source.char_indices() {
        if utf16_pos == start {
            start_byte = Some(byte_idx);
        }
        utf16_pos += ch.len_utf16();
        if utf16_pos == end {
            end_byte = Some(byte_idx + ch.len_utf8());
            break;
        }
    }

    let total_utf16 = source.encode_utf16().count();
    if start == total_utf16 {
        start_byte = Some(source.len());
    }
    if end == total_utf16 {
        end_byte = Some(source.len());
    }

    let start_byte = start_byte.unwrap_or(0);
    let end_byte = end_byte.unwrap_or(start_byte);
    source[start_byte..end_byte].to_string()
}

fn utf16_len(source: &str) -> u32 {
    source.encode_utf16().count() as u32
}

fn error_message(err: JsValue) -> Option<String> {
    if let Some(message) = err.as_string() {
        return Some(message);
    }
    Reflect::get(&err, &JsValue::from_str("message"))
        .ok()
        .and_then(|v| v.as_string())
}

#[wasm_bindgen_test]
fn analyze_ascii_spans_and_output_type() {
    let source = "1+2";
    let result = analyze_value(source);

    assert_eq!(result.output_type, "number");

    let kinds: Vec<&str> = result.tokens.iter().map(|t| t.kind.as_str()).collect();
    assert_eq!(kinds, vec!["Number", "Plus", "Number", "Eof"]);

    let first = &result.tokens[0];
    assert_eq!(first.span.start, 0);
    assert_eq!(first.span.end, 1);

    let plus = &result.tokens[1];
    assert_eq!(plus.span.start, 1);
    assert_eq!(plus.span.end, 2);

    for token in &result.tokens {
        if token.text.is_empty() {
            continue;
        }
        let slice = utf16_slice(source, token.span.start as usize, token.span.end as usize);
        assert_eq!(slice, token.text);
    }
}

#[wasm_bindgen_test]
fn analyze_chinese_spans() {
    let source = "变量+1";
    let result = analyze_value(source);

    let ident = &result.tokens[0];
    assert_eq!(ident.kind, "Ident");
    assert_eq!(ident.span.start, 0);
    assert_eq!(ident.span.end, 2);

    let plus = &result.tokens[1];
    assert_eq!(plus.kind, "Plus");
    assert_eq!(plus.span.start, 2);
    assert_eq!(plus.span.end, 3);

    for token in &result.tokens {
        if token.text.is_empty() {
            continue;
        }
        let slice = utf16_slice(source, token.span.start as usize, token.span.end as usize);
        assert_eq!(slice, token.text);
    }
}

#[wasm_bindgen_test]
fn analyze_emoji_spans_and_diagnostics() {
    let source = "😀+1";
    let result = analyze_value(source);

    let ident = &result.tokens[0];
    assert_eq!(ident.kind, "Ident");
    assert_eq!(ident.span.start, 0);
    assert_eq!(ident.span.end, 2);

    let plus = &result.tokens[1];
    assert_eq!(plus.kind, "Plus");
    assert_eq!(plus.span.start, 2);
    assert_eq!(plus.span.end, 3);

    let error_source = "1 +";
    let error_result = analyze_value(error_source);
    assert!(!error_result.diagnostics.is_empty());
    assert_eq!(error_result.output_type, "unknown");

    let diag = &error_result.diagnostics[0];
    assert_eq!(diag.kind, "error");
    assert_eq!(diag.span.start, 2);
    assert_eq!(diag.span.end, 3);
}

#[wasm_bindgen_test]
fn analyze_diagnostics_include_actions() {
    let source = "f(1 2)";
    let result = analyze_value(source);
    let with_actions = result
        .diagnostics
        .iter()
        .find(|diag| !diag.actions.is_empty())
        .expect("expected diagnostic with action");

    assert!(with_actions.actions.iter().any(|action| {
        action.title == "Insert `,`"
            && action
                .edits
                .iter()
                .any(|e| e.range.start == 4 && e.range.end == 4 && e.new_text == ",")
    }));
}

#[wasm_bindgen_test]
fn analyze_diagnostics_include_line_col_multiline() {
    let source = "1 +\n2 *";
    let result = analyze_value(source);
    let diag = result
        .diagnostics
        .first()
        .expect("expected diagnostic for incomplete expression");

    assert_eq!(diag.kind, "error");
    assert_eq!(diag.span.start, 6);
    assert_eq!(diag.span.end, 7);
    assert_eq!(diag.line, 2);
    assert_eq!(diag.col, 3);
}

#[wasm_bindgen_test]
fn format_success_returns_source_and_cursor() {
    let source = "1+2";
    let cursor = 2;
    let out = format_value(source, cursor);

    assert!(!out.source.is_empty());
    assert!(out.cursor <= utf16_len(&out.source));
}

#[wasm_bindgen_test]
fn format_rebases_mid_document_cursor_through_full_replace_edit() {
    let source = "1+2";
    let out = format_value(source, 1);
    assert_eq!(out.cursor, 0);
}

#[wasm_bindgen_test]
fn format_parse_error_returns_err() {
    let source = "1 +";
    let err = analyzer(None)
        .ide_format(source.to_string(), 0)
        .expect_err("expected ide_format() Err");
    assert_eq!(error_message(err).as_deref(), Some("Format error"));
}

#[wasm_bindgen_test]
fn format_lex_error_returns_err() {
    let source = "1 @";
    let err = analyzer(None)
        .ide_format(source.to_string(), 0)
        .expect_err("expected ide_format() Err");
    assert_eq!(error_message(err).as_deref(), Some("Format error"));
}

#[wasm_bindgen_test]
fn apply_edits_changes_source_and_returns_cursor() {
    let source = "abc";
    let out = apply_edits_value(source, &[edit(1, 2, "X")], 2);

    assert_eq!(out.source, "aXc");
    assert!(out.cursor <= utf16_len(&out.source));
}

#[wasm_bindgen_test]
fn apply_edits_overlapping_returns_err() {
    let source = "abcd";
    let edits: JsValue = serde_wasm_bindgen::to_value(&vec![edit(1, 3, "X"), edit(2, 4, "Y")])
        .expect("edits to JsValue");

    let err = analyzer(None)
        .ide_apply_edits(source.to_string(), edits, 0)
        .expect_err("expected overlapping edits Err");
    assert_eq!(error_message(err).as_deref(), Some("Overlapping edits"));
}

#[wasm_bindgen_test]
fn apply_edits_invalid_range_returns_err() {
    let source = "abcd";
    let edits: JsValue =
        serde_wasm_bindgen::to_value(&vec![edit(5, 5, "X")]).expect("edits to JsValue");

    let err = analyzer(None)
        .ide_apply_edits(source.to_string(), edits, 0)
        .expect_err("expected invalid range Err");
    assert_eq!(error_message(err).as_deref(), Some("Invalid edit range"));
}

#[wasm_bindgen_test]
fn apply_edits_emoji_utf16_conversion_is_correct() {
    let source = "😀a";
    let edits: JsValue =
        serde_wasm_bindgen::to_value(&vec![edit(2, 3, "Z")]).expect("edits to JsValue");

    let out = analyzer(None)
        .ide_apply_edits(source.to_string(), edits, 2)
        .expect("expected ide_apply_edits() Ok");
    let out: ApplyResult = serde_wasm_bindgen::from_value(out).expect("ApplyResult");

    assert_eq!(out.source, "😀Z");
    assert_eq!(out.cursor, 2);
}

#[wasm_bindgen_test]
fn analyzer_new_rejects_non_object_config() {
    let err = analyzer_wasm::Analyzer::new(JsValue::from_str("{"))
        .err()
        .expect("expected Analyzer::new Err on invalid config");
    assert_eq!(err, "Invalid analyzer config");
}

#[wasm_bindgen_test]
fn analyzer_new_rejects_unknown_fields() {
    let config = js_sys::Object::new();
    Reflect::set(
        &config,
        &JsValue::from_str("functions"),
        &js_sys::Array::new().into(),
    )
    .expect("set functions");
    let err = analyzer_wasm::Analyzer::new(config.into())
        .err()
        .expect("expected Analyzer::new Err on unknown fields");
    assert_eq!(err, "Invalid analyzer config");
}

#[wasm_bindgen_test]
fn analyzer_new_rejects_invalid_properties_structure() {
    let config = js_sys::Object::new();
    Reflect::set(
        &config,
        &JsValue::from_str("properties"),
        &js_sys::Object::new().into(),
    )
    .expect("set properties");
    let err = analyzer_wasm::Analyzer::new(config.into())
        .err()
        .expect("expected Analyzer::new Err on invalid properties");
    assert_eq!(err, "Invalid analyzer config");
}

#[wasm_bindgen_test]
fn analyzer_config_nullable_preferred_limit_defaults_to_five() {
    let source = "if(";

    let out_null = analyzer(None)
        .ide_help(source.to_string(), 3)
        .expect("expected ide_help() Ok");
    let out_null: HelpResult = serde_wasm_bindgen::from_value(out_null).expect("HelpResult");

    let out_five = analyzer(Some(5))
        .ide_help(source.to_string(), 3)
        .expect("expected ide_help() Ok");
    let out_five: HelpResult = serde_wasm_bindgen::from_value(out_five).expect("HelpResult");

    assert_eq!(
        out_null.completion.preferred_indices,
        out_five.completion.preferred_indices
    );
}

#[wasm_bindgen_test]
fn analyzer_config_zero_preferred_limit_disables_preferred_indices() {
    let out = analyzer(Some(0))
        .ide_help("if(".to_string(), 3)
        .expect("expected ide_help() Ok");
    let out: HelpResult = serde_wasm_bindgen::from_value(out).expect("HelpResult");

    assert!(out.completion.preferred_indices.is_empty());
}
