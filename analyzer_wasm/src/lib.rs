//! WASM entry points for the analyzer.
//!
//! Core code uses UTF-8 byte offsets. The JS boundary uses UTF-16 code unit offsets.
//! Spans are half-open `[start, end)`.
mod converter;
pub mod dto;
mod offsets;
mod span;

use analyzer::{Span as ByteSpan, TextEdit as ByteTextEdit};
use js_sys::Error as JsError;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::converter::Converter;
use crate::dto::v1::{AnalyzeResult, ApplyResult, Span as Utf16Span, TextEdit as Utf16TextEdit};

#[wasm_bindgen]
pub fn analyze(source: String, context_json: String) -> Result<JsValue, JsValue> {
    let parsed = Converter::parse_context(&context_json)?;
    let result = analyzer::analyze(&source, &parsed.ctx);
    let out: AnalyzeResult = Converter::analyze_output(&source, result);
    to_js_value(&out)
}

#[wasm_bindgen]
pub fn ide_format(source: String, cursor_utf16: u32) -> Result<JsValue, JsValue> {
    let cursor_byte = cursor_utf16_to_valid_byte(&source, cursor_utf16)? as u32;
    let output = analyzer::ide_format(&source, cursor_byte).map_err(ide_error_to_js)?;
    to_utf16_apply_result(output)
}

#[wasm_bindgen]
pub fn ide_apply_edits(
    source: String,
    edits: JsValue,
    cursor_utf16: u32,
) -> Result<JsValue, JsValue> {
    let edits_view: Vec<Utf16TextEdit> = serde_wasm_bindgen::from_value(edits)
        .map_err(|_| JsValue::from(JsError::new("Invalid edits")))?;

    let byte_edits = text_edits_utf16_to_byte(&source, edits_view)?;
    let cursor_byte = cursor_utf16_to_valid_byte(&source, cursor_utf16)? as u32;
    let output =
        analyzer::ide_apply_edits(&source, byte_edits, cursor_byte).map_err(ide_error_to_js)?;
    to_utf16_apply_result(output)
}

#[wasm_bindgen]
pub fn ide_help(source: String, cursor: usize, context_json: String) -> Result<JsValue, JsValue> {
    let cursor_byte = Converter::utf16_offset_to_byte(&source, cursor);
    let parsed = Converter::parse_context(&context_json)?;
    let output = analyzer::ide_help(&source, cursor_byte, &parsed.ctx, parsed.completion);
    let out = Converter::help_output_view(&source, &output);
    to_js_value(&out)
}

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

fn to_utf16_apply_result(output: analyzer::IdeApplyResult) -> Result<JsValue, JsValue> {
    let out = ApplyResult {
        cursor: Converter::byte_offset_to_utf16_offset(&output.source, output.cursor as usize),
        source: output.source,
    };
    to_js_value(&out)
}

fn ide_error_to_js(err: analyzer::IdeError) -> JsValue {
    JsValue::from(JsError::new(err.message()))
}

fn cursor_utf16_to_valid_byte(source: &str, cursor_utf16: u32) -> Result<usize, JsValue> {
    let utf16_len = source.encode_utf16().count();
    let cursor_utf16 = cursor_utf16 as usize;
    if cursor_utf16 > utf16_len {
        return Err(JsValue::from(JsError::new("Invalid cursor")));
    }

    let cursor_byte = Converter::utf16_offset_to_byte(source, cursor_utf16);
    if !source.is_char_boundary(cursor_byte) {
        return Err(JsValue::from(JsError::new("Invalid cursor")));
    }

    Ok(cursor_byte)
}

fn text_edits_utf16_to_byte(
    source: &str,
    edits: Vec<Utf16TextEdit>,
) -> Result<Vec<ByteTextEdit>, JsValue> {
    let utf16_len = source.encode_utf16().count();

    let mut byte_edits = Vec::with_capacity(edits.len());
    for edit in edits {
        let Utf16Span { start, end } = edit.range;
        let start_utf16 = start as usize;
        let end_utf16 = end as usize;

        if end_utf16 < start_utf16 || end_utf16 > utf16_len {
            return Err(JsValue::from(JsError::new("Invalid edit range")));
        }

        let start_byte = Converter::utf16_offset_to_byte(source, start_utf16);
        let end_byte = Converter::utf16_offset_to_byte(source, end_utf16);

        if end_byte < start_byte {
            return Err(JsValue::from(JsError::new("Invalid edit range")));
        }
        if !source.is_char_boundary(start_byte) || !source.is_char_boundary(end_byte) {
            return Err(JsValue::from(JsError::new("Invalid edit range")));
        }

        byte_edits.push(ByteTextEdit {
            range: ByteSpan {
                start: start_byte as u32,
                end: end_byte as u32,
            },
            new_text: edit.new_text,
        });
    }

    Ok(byte_edits)
}
