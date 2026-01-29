//! WASM entry points for `notion-formula-rs`.
//!
//! This crate exposes a small, deterministic surface to JS via `wasm-bindgen`.
//! The analyzer core works in **UTF-8 byte offsets**, while the JS/editor boundary uses
//! **UTF-16 code unit offsets** (CodeMirror-style positions). All ranges/spans are **half-open**
//! `[start, end)` (inclusive start, exclusive end) across the boundary.
//!
//! **Entry points**
//! - [`analyze`]: parse/semantic analysis + formatting + token/diagnostic views.
//! - [`complete`]: code completion at a UTF-16 cursor position.
//! - [`pos_to_line_col`]: map a UTF-16 position to a 1-based (line, col) view.
mod converter;
pub mod dto;
mod offsets;
mod span;
mod text_edit;

use js_sys::Error as JsError;
use wasm_bindgen::prelude::*;

use crate::converter::Converter;
use crate::dto::v1::{AnalyzeResult, LineColView};

/// Analyze `source` and return a serialized [`dto::v1::AnalyzeResult`].
///
/// **Units / encoding boundary**
/// - `source` is a Rust `String` (UTF-8).
/// - All spans in the returned DTO are **UTF-16 code unit offsets** into `source`, with half-open
///   ranges `[start, end)`.
///
/// **Semantics**
/// - Parser errors and semantic diagnostics are returned *in-band* as `diagnostics` (they do not
///   throw).
/// - Tokens are non-trivia tokens from the parse.
/// - `formatted` is the formatter output when parsing succeeds; on a hard parse error it is an
///   empty string.
///
/// **Error model**
/// - Invalid `context_json` becomes a thrown `JsValue` error (`"Invalid context JSON"`).
/// - DTO serialization failure becomes a thrown `JsValue` error (`"Serialize error"`).
#[wasm_bindgen]
pub fn analyze(source: String, context_json: String) -> Result<JsValue, JsValue> {
    // Parse the context from the provided JSON string.
    let parsed = Converter::parse_context(&context_json)?;

    // Perform the analysis and collect the diagnostics.
    let result: AnalyzeResult = match analyzer::analyze(&source) {
        Ok(mut output) => {
            // Analyze the expression and append the diagnostics.
            let (_, diags) = analyzer::semantic::analyze_expr(&output.expr, &parsed.ctx);
            output.diagnostics.extend(diags);
            // Convert the output to the desired DTO format.
            Converter::analyze_output(&source, output)
        }
        Err(diag) => {
            // If an error occurs, convert the error into a DTO.
            Converter::analyze_error(&source, &diag)
        }
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

/// Compute completion items at `cursor` and return a serialized [`dto::v1::CompletionOutputView`].
///
/// **Units / encoding boundary**
/// - `cursor` is a **UTF-16 code unit offset** into `source` (CodeMirror-style).
/// - The cursor is converted to a **UTF-8 byte offset** for the core analyzer; if it falls inside a
///   surrogate pair (or beyond the end), it is deterministically floored/clamped to a valid
///   position.
/// - All spans/cursors in the returned DTO are **UTF-16 code unit offsets** with half-open ranges
///   `[start, end)`.
///
/// **Semantics**
/// - The completion engine runs against the parsed `context_json` (properties + config).
/// - Per-item edits (and optional per-item cursors) are reported in the *editor* coordinate space
///   (UTF-16) and are intended to be applied by the caller.
///
/// **Error model**
/// - Invalid `context_json` becomes a thrown `JsValue` error (`"Invalid context JSON"`).
/// - DTO serialization failure becomes a thrown `JsValue` error (`"Serialize error"`).
#[wasm_bindgen]
pub fn complete(source: String, cursor: usize, context_json: String) -> Result<JsValue, JsValue> {
    // Convert the cursor position from UTF-16 to byte offset.
    let cursor_byte = Converter::cursor_utf16_to_byte(&source, cursor);

    // Parse the context from the provided JSON string.
    let parsed = Converter::parse_context(&context_json)?;

    // Perform the completion operation.
    let output =
        analyzer::completion::complete(&source, cursor_byte, Some(&parsed.ctx), parsed.completion);

    // Convert the completion output to the desired DTO format.
    let out = Converter::completion_output_view(&source, &output);
    serde_wasm_bindgen::to_value(&out).map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

/// Convert a UTF-16 `pos` into a 1-based (line, col) view.
///
/// **Units / encoding boundary**
/// - `pos` is a **UTF-16 code unit offset** into `source`.
/// - `pos` is first converted to a **UTF-8 byte offset** using the same UTF-16→byte conversion
///   rules used elsewhere at the WASM boundary: it is clamped to the UTF-16 length of the string
///   and deterministically floored if it falls inside a Unicode scalar's UTF-16 encoding (e.g.
///   inside a surrogate pair).
/// - The resulting byte offset is then clamped to a valid UTF-8 char boundary by `SourceMap`.
///
/// **Semantics**
/// - Returns a serialized [`dto::v1::LineColView`] with both `line` and `col` being **1-based**.
///
/// **Error model**
/// - This function does not throw.
/// - If serialization fails, it returns `JsValue::NULL` (JS `null`).
#[wasm_bindgen]
pub fn pos_to_line_col(source: String, pos: u32) -> JsValue {
    // Convert the position to a line and column view.
    let out: LineColView = Converter::pos_to_line_col_view(&source, pos);

    serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
}
