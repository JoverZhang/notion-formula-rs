#![cfg(target_arch = "wasm32")]

use analyzer_wasm::{Analyzer, FormulaEngineSession};
use js_sys::{Array, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

fn field(value: &JsValue, name: &str) -> JsValue {
    Reflect::get(value, &JsValue::from_str(name)).unwrap()
}

fn analyzer() -> Analyzer {
    let config = js_sys::Object::new();
    Reflect::set(&config, &JsValue::from_str("properties"), &Array::new()).unwrap();
    Analyzer::new(config.into()).unwrap()
}

#[wasm_bindgen_test]
fn analyzer_tokens_keep_raw_text_and_utf16_spans_and_expose_decoded_values() {
    let analyzer = analyzer();
    for (source, expected) in [
        (
            r#""line\nnext\t\"quote\"\\path""#,
            "line\nnext\t\"quote\"\\path",
        ),
        (r#""""#, ""),
        ("\"中文😀\"", "中文😀"),
        ("\"raw\r\0\u{1}\"", "raw\r\0\u{1}"),
    ] {
        let result = analyzer.analyze(source.into()).unwrap();
        let tokens = Array::from(&field(&result, "tokens"));
        let token = tokens.get(0);
        assert_eq!(field(&token, "kind").as_string().as_deref(), Some("String"));
        assert_eq!(field(&token, "text").as_string().as_deref(), Some(source));
        assert_eq!(
            field(&token, "string_value").as_string().as_deref(),
            Some(expected)
        );
        let span = field(&token, "span");
        assert_eq!(field(&span, "start").as_f64(), Some(0.0));
        assert_eq!(
            field(&span, "end").as_f64(),
            Some(source.encode_utf16().count() as f64)
        );
        assert!(field(&tokens.get(1), "string_value").is_null());
    }
}

#[wasm_bindgen_test]
fn analyzer_serializes_absent_string_values_as_present_null_fields() {
    let analyzer = analyzer();
    for source in [
        r#""bad\q""#,
        r#""bad\r""#,
        r#""bad\u0000""#,
        "identifier + 1 + true",
        "\"unfinished",
    ] {
        let result = analyzer.analyze(source.into()).unwrap();
        let tokens = Array::from(&field(&result, "tokens"));
        for token in tokens {
            assert!(Reflect::has(&token, &JsValue::from_str("string_value")).unwrap());
            assert!(field(&token, "string_value").is_null(), "{source:?}");
        }
    }
}

#[wasm_bindgen_test]
fn draft_tokens_share_string_values_and_measure_offsets_after_emoji_in_utf16() {
    let schema = serde_wasm_bindgen::to_value(&analyzer_wasm::dto::engine::FormulaSchema {
        properties: Vec::new(),
    })
    .unwrap();
    let mut session = FormulaEngineSession::new(schema).unwrap();
    let source = "\"😀\" + prop(\"quote\\\"slash\\\\\")";
    let definition = serde_wasm_bindgen::to_value(&analyzer_wasm::dto::engine::FormulaDefinition {
        id: "draft".into(),
        expression: source.into(),
    })
    .unwrap();
    let handle = session.create_draft(definition).unwrap();
    let state = session.draft_state(handle).unwrap();
    let tokens = Array::from(&field(&state, "tokens"));
    let string = tokens.get(4);
    assert_eq!(
        field(&string, "string_value").as_string().as_deref(),
        Some("quote\"slash\\")
    );
    assert_eq!(field(&field(&string, "span"), "start").as_f64(), Some(12.0));
    assert!(field(&tokens.get(1), "string_value").is_null());
}
