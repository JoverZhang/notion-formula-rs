//! WASM entry points for the analyzer.
//!
//! Core code uses UTF-8 byte offsets. The JS boundary uses UTF-16 code unit offsets.
//! Spans are half-open `[start, end)`.
mod converter;
pub mod dto;
mod offsets;
mod span;
mod text_edit;

use js_sys::Error as JsError;
use wasm_bindgen::prelude::*;

use crate::converter::Converter;
use crate::dto::v1::{AnalyzeResult, LineColView};

/// Analyze `source` and return a [`AnalyzeResult`].
///
/// Spans in the result use UTF-16 code units and are half-open `[start, end)`.
///
/// Invalid `context_json` or a serialization error is returned as a thrown `JsValue`.
/// Diagnostics are returned in the payload; parse/semantic errors do not throw.
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

/// Compute completion items at `cursor` (UTF-16 code units) and return a DTO view.
/// Edits are in original-document coordinates (UTF-16).
///
/// `cursor` is converted to a byte offset for the core analyzer, with clamping/flooring to a
/// valid boundary. The result uses UTF-16 spans and edits.
/// Invalid `context_json` or a serialization error is returned as a thrown `JsValue`.
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

/// Convert `pos` (UTF-16 code units) to a 1-based `(line, col)` view.
///
/// The position is clamped/floored when converting to a byte offset, then clamped to a UTF-8 char
/// boundary by `SourceMap`. This function does not throw; on serialization failure it returns
/// `JsValue::NULL`.
#[wasm_bindgen]
pub fn pos_to_line_col(source: String, pos: u32) -> JsValue {
    // Convert the position to a line and column view.
    let out: LineColView = Converter::pos_to_line_col_view(&source, pos);

    serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
}
