use serde::Deserialize;
use wasm_bindgen_test::wasm_bindgen_test;

#[derive(Deserialize)]
struct AnalyzeResult {
    diagnostics: Vec<Diagnostic>,
    tokens: Vec<TokenView>,
    formatted: String,
}

#[derive(Deserialize)]
struct Span {
    start: usize,
    end: usize,
    line: usize,
    col: usize,
}

#[derive(Deserialize)]
struct Diagnostic {
    kind: String,
    _message: String,
    span: Span,
}

#[derive(Deserialize)]
struct TokenView {
    kind: String,
    text: String,
    span: Span,
}

fn analyze_value(source: &str) -> AnalyzeResult {
    let value = analyzer_wasm::analyze(source.to_string(), None);
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

    let kinds: Vec<&str> = result.tokens.iter().map(|t| t.kind.as_str()).collect();
    assert_eq!(kinds, vec!["Number", "Plus", "Number", "Eof"]);

    let first = &result.tokens[0];
    assert_eq!(first.span.start, 0);
    assert_eq!(first.span.end, 1);
    assert_eq!(first.span.line, 1);
    assert_eq!(first.span.col, 1);

    let plus = &result.tokens[1];
    assert_eq!(plus.span.start, 1);
    assert_eq!(plus.span.end, 2);
    assert_eq!(plus.span.line, 1);
    assert_eq!(plus.span.col, 2);

    for token in &result.tokens {
        if token.text.is_empty() {
            continue;
        }
        let slice = utf16_slice(source, token.span.start, token.span.end);
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
    assert_eq!(ident.span.line, 1);
    assert_eq!(ident.span.col, 1);

    let plus = &result.tokens[1];
    assert_eq!(plus.kind, "Plus");
    assert_eq!(plus.span.start, 2);
    assert_eq!(plus.span.end, 3);
    assert_eq!(plus.span.line, 1);
    assert_eq!(plus.span.col, 3);

    for token in &result.tokens {
        if token.text.is_empty() {
            continue;
        }
        let slice = utf16_slice(source, token.span.start, token.span.end);
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
    assert_eq!(ident.span.line, 1);
    assert_eq!(ident.span.col, 1);

    let plus = &result.tokens[1];
    assert_eq!(plus.kind, "Plus");
    assert_eq!(plus.span.start, 2);
    assert_eq!(plus.span.end, 3);
    assert_eq!(plus.span.line, 1);
    assert_eq!(plus.span.col, 2);

    let error_source = "1 +";
    let error_result = analyze_value(error_source);
    assert!(!error_result.diagnostics.is_empty());

    let diag = &error_result.diagnostics[0];
    assert_eq!(diag.kind, "error");
    assert_eq!(diag.span.start, 2);
    assert_eq!(diag.span.end, 3);
    assert_eq!(diag.span.line, 1);
    assert_eq!(diag.span.col, 3);
}
