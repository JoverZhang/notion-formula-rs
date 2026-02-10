use serde::Deserialize;
use wasm_bindgen_test::wasm_bindgen_test;

#[derive(Deserialize)]
struct AnalyzeResult {
    diagnostics: Vec<Diagnostic>,
    tokens: Vec<TokenView>,
    formatted: String,
    output_type: String,
}

#[derive(Deserialize)]
struct Span {
    start: u32,
    end: u32,
}

#[derive(Deserialize)]
struct SpanView {
    range: Span,
}

#[derive(Deserialize)]
struct Diagnostic {
    kind: String,
    _message: String,
    span: SpanView,
}

#[derive(Deserialize)]
struct TokenView {
    kind: String,
    text: String,
    span: SpanView,
}

fn analyze_value(source: &str) -> AnalyzeResult {
    let value = analyzer_wasm::analyze(source.to_string(), r#"{}"#.to_string())
        .expect("expected analyze() Ok");
    serde_wasm_bindgen::from_value(value).expect("expected AnalyzeResult")
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

#[wasm_bindgen_test]
fn analyze_ascii_spans_and_format() {
    let source = "1+2";
    let result = analyze_value(source);

    assert!(!result.formatted.is_empty());
    assert_eq!(result.output_type, "number");

    let kinds: Vec<&str> = result.tokens.iter().map(|t| t.kind.as_str()).collect();
    assert_eq!(kinds, vec!["Number", "Plus", "Number", "Eof"]);

    let first = &result.tokens[0];
    assert_eq!(first.span.range.start, 0);
    assert_eq!(first.span.range.end, 1);

    let plus = &result.tokens[1];
    assert_eq!(plus.span.range.start, 1);
    assert_eq!(plus.span.range.end, 2);

    for token in &result.tokens {
        if token.text.is_empty() {
            continue;
        }
        let slice = utf16_slice(
            source,
            token.span.range.start as usize,
            token.span.range.end as usize,
        );
        assert_eq!(slice, token.text);
    }
}

#[wasm_bindgen_test]
fn analyze_chinese_spans() {
    let source = "变量+1";
    let result = analyze_value(source);

    let ident = &result.tokens[0];
    assert_eq!(ident.kind, "Ident");
    assert_eq!(ident.span.range.start, 0);
    assert_eq!(ident.span.range.end, 2);

    let plus = &result.tokens[1];
    assert_eq!(plus.kind, "Plus");
    assert_eq!(plus.span.range.start, 2);
    assert_eq!(plus.span.range.end, 3);

    for token in &result.tokens {
        if token.text.is_empty() {
            continue;
        }
        let slice = utf16_slice(
            source,
            token.span.range.start as usize,
            token.span.range.end as usize,
        );
        assert_eq!(slice, token.text);
    }
}

#[wasm_bindgen_test]
fn analyze_emoji_spans_and_diagnostics() {
    let source = "😀+1";
    let result = analyze_value(source);

    let ident = &result.tokens[0];
    assert_eq!(ident.kind, "Ident");
    assert_eq!(ident.span.range.start, 0);
    assert_eq!(ident.span.range.end, 2);

    let plus = &result.tokens[1];
    assert_eq!(plus.kind, "Plus");
    assert_eq!(plus.span.range.start, 2);
    assert_eq!(plus.span.range.end, 3);

    let error_source = "1 +";
    let error_result = analyze_value(error_source);
    assert!(!error_result.diagnostics.is_empty());
    assert_eq!(error_result.output_type, "unknown");

    let diag = &error_result.diagnostics[0];
    assert_eq!(diag.kind, "error");
    assert_eq!(diag.span.range.start, 2);
    assert_eq!(diag.span.range.end, 3);
}

#[wasm_bindgen_test]
fn analyze_invalid_context_errors() {
    let source = "1+2";
    let err = analyzer_wasm::analyze(source.to_string(), "{".to_string())
        .expect_err("expected analyze() Err on invalid context JSON");
    assert_eq!(err.as_string().as_deref(), Some("Invalid context JSON"));
}

#[wasm_bindgen_test]
fn analyze_empty_context_errors() {
    let source = "1+2";
    let err = analyzer_wasm::analyze(source.to_string(), "   ".to_string())
        .expect_err("expected analyze() Err on empty context JSON");
    assert_eq!(err.as_string().as_deref(), Some("Invalid context JSON"));
}

#[wasm_bindgen_test]
fn analyze_rejects_functions_in_context_json() {
    let source = "1+2";
    let err = analyzer_wasm::analyze(source.to_string(), r#"{"functions":[]}"#.to_string())
        .expect_err("expected analyze() Err on unknown context JSON fields");
    assert_eq!(err.as_string().as_deref(), Some("Invalid context JSON"));
}

#[wasm_bindgen_test]
fn analyze_rejects_invalid_properties_structure() {
    let source = "1+2";
    let err = analyzer_wasm::analyze(source.to_string(), r#"{"properties":{}}"#.to_string())
        .expect_err("expected analyze() Err on invalid context JSON structure");
    assert_eq!(err.as_string().as_deref(), Some("Invalid context JSON"));
}

#[wasm_bindgen_test]
fn complete_invalid_context_errors() {
    let source = "1+2";
    let err = analyzer_wasm::complete(source.to_string(), 0, "{".to_string())
        .expect_err("expected complete() Err on invalid context JSON");
    assert_eq!(err.as_string().as_deref(), Some("Invalid context JSON"));
}
