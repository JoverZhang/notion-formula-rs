//! WASM entry points for the analyzer.
//!
//! Core code uses UTF-8 byte offsets. The JS boundary uses UTF-16 code unit offsets.
//! Spans are half-open `[start, end)`.
mod converter;
pub mod dto;
mod offsets;
mod span;
mod text_edit;

use analyzer::{DiagnosticCode, Span, TextEdit};
use js_sys::Error as JsError;
use wasm_bindgen::prelude::*;

use crate::converter::Converter;
use crate::dto::v1::{AnalyzeResult, ApplyResultView, TextEditView};
use crate::offsets::{byte_offset_to_utf16_offset, utf16_offset_to_byte};
use crate::text_edit::apply_text_edits_bytes_with_cursor;

#[wasm_bindgen]
pub fn analyze(source: String, context_json: String) -> Result<JsValue, JsValue> {
    let parsed = Converter::parse_context(&context_json)?;

    let result: AnalyzeResult = match analyzer::analyze(&source) {
        Ok(mut output) => {
            let (ty, diags) = analyzer::semantic::analyze_expr(&output.expr, &parsed.ctx);
            output.diagnostics.extend(diags);
            Converter::analyze_output(&source, output, ty.to_string())
        }
        Err(diag) => Converter::analyze_error(&source, &diag),
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

#[wasm_bindgen]
pub fn format(source: String, cursor_utf16: u32) -> Result<JsValue, JsValue> {
    let output =
        analyzer::analyze(&source).map_err(|_| JsValue::from(JsError::new("Format error")))?;

    if has_syntax_errors(&output.diagnostics) {
        return Err(JsValue::from(JsError::new("Format error")));
    }

    // Formatting replaces the entire document. Preserve caller cursor semantics by validating
    // against the input, then clamping in UTF-16 space to the formatted output length.
    let _ = cursor_utf16_to_valid_byte(&source, cursor_utf16)?;
    let formatted = analyzer::format_expr(&output.expr, &source, &output.tokens);
    let out = ApplyResultView {
        cursor: cursor_utf16.min(formatted.encode_utf16().count() as u32),
        source: formatted,
    };

    serde_wasm_bindgen::to_value(&out).map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

#[wasm_bindgen]
pub fn apply_edits(source: String, edits: JsValue, cursor_utf16: u32) -> Result<JsValue, JsValue> {
    let edits_view: Vec<TextEditView> = serde_wasm_bindgen::from_value(edits)
        .map_err(|_| JsValue::from(JsError::new("Invalid edits")))?;

    let byte_edits = text_edits_utf16_to_sorted_byte(&source, edits_view)?;
    apply_sorted_byte_edits(&source, byte_edits, cursor_utf16)
}

#[wasm_bindgen]
pub fn complete(source: String, cursor: usize, context_json: String) -> Result<JsValue, JsValue> {
    let cursor_byte = Converter::cursor_utf16_to_byte(&source, cursor);
    let parsed = Converter::parse_context(&context_json)?;

    let output =
        analyzer::completion::complete(&source, cursor_byte, Some(&parsed.ctx), parsed.completion);

    let out = Converter::completion_output_view(&source, &output);
    serde_wasm_bindgen::to_value(&out).map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

fn has_syntax_errors(diagnostics: &[analyzer::Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|d| matches!(d.code, DiagnosticCode::LexError | DiagnosticCode::Parse(_)))
}

fn apply_sorted_byte_edits(
    source: &str,
    edits: Vec<TextEdit>,
    cursor_utf16: u32,
) -> Result<JsValue, JsValue> {
    let cursor_byte = cursor_utf16_to_valid_byte(source, cursor_utf16)?;
    validate_sorted_non_overlapping_edits(source, &edits)?;

    let (updated_source, cursor_byte_after) =
        apply_text_edits_bytes_with_cursor(source, &edits, cursor_byte as u32);

    let out = ApplyResultView {
        source: updated_source.clone(),
        cursor: byte_offset_to_utf16_offset(&updated_source, cursor_byte_after as usize),
    };

    serde_wasm_bindgen::to_value(&out).map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

fn cursor_utf16_to_valid_byte(source: &str, cursor_utf16: u32) -> Result<usize, JsValue> {
    let utf16_len = source.encode_utf16().count();
    let cursor_utf16 = cursor_utf16 as usize;
    if cursor_utf16 > utf16_len {
        return Err(JsValue::from(JsError::new("Invalid cursor")));
    }

    let cursor_byte = utf16_offset_to_byte(source, cursor_utf16);
    if !source.is_char_boundary(cursor_byte) {
        return Err(JsValue::from(JsError::new("Invalid cursor")));
    }

    Ok(cursor_byte)
}

fn text_edits_utf16_to_sorted_byte(
    source: &str,
    edits: Vec<TextEditView>,
) -> Result<Vec<TextEdit>, JsValue> {
    let utf16_len = source.encode_utf16().count();

    let mut byte_edits = Vec::with_capacity(edits.len());
    for edit in edits {
        let start_utf16 = edit.range.start as usize;
        let end_utf16 = edit.range.end as usize;

        if end_utf16 < start_utf16 || end_utf16 > utf16_len {
            return Err(JsValue::from(JsError::new("Invalid edit range")));
        }

        let start_byte = utf16_offset_to_byte(source, start_utf16);
        let end_byte = utf16_offset_to_byte(source, end_utf16);

        if end_byte < start_byte {
            return Err(JsValue::from(JsError::new("Invalid edit range")));
        }
        if !source.is_char_boundary(start_byte) || !source.is_char_boundary(end_byte) {
            return Err(JsValue::from(JsError::new("Invalid edit range")));
        }

        byte_edits.push(TextEdit {
            range: Span {
                start: start_byte as u32,
                end: end_byte as u32,
            },
            new_text: edit.new_text,
        });
    }

    byte_edits.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then(a.range.end.cmp(&b.range.end))
    });

    validate_sorted_non_overlapping_edits(source, &byte_edits)?;
    Ok(byte_edits)
}

fn validate_sorted_non_overlapping_edits(source: &str, edits: &[TextEdit]) -> Result<(), JsValue> {
    let mut prev_end = 0u32;
    let source_len = source.len() as u32;

    for (index, edit) in edits.iter().enumerate() {
        if edit.range.end < edit.range.start || edit.range.end > source_len {
            return Err(JsValue::from(JsError::new("Invalid edit range")));
        }

        if !source.is_char_boundary(edit.range.start as usize)
            || !source.is_char_boundary(edit.range.end as usize)
        {
            return Err(JsValue::from(JsError::new("Invalid edit range")));
        }

        if index > 0 && edit.range.start < prev_end {
            return Err(JsValue::from(JsError::new("Overlapping edits")));
        }

        prev_end = edit.range.end;
    }

    Ok(())
}
