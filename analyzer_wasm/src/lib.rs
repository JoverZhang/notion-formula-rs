mod converter;
pub mod dto;
mod offsets;
mod span;
mod text_edit;

use js_sys::Error as JsError;
use wasm_bindgen::prelude::*;

use crate::converter::Converter;
use crate::dto::v1::{AnalyzeResult, LineColView};

/// Analyze a source string and return a serialized `AnalyzeResult` or an error.
///
/// # Arguments:
/// * `source` - The source string to analyze.
/// * `context_json` - A JSON string representing the context, used to influence the analysis.
///
/// # Returns:
/// A `JsValue` representing the serialized result of the analysis, or an error if the serialization fails.
#[wasm_bindgen]
pub fn analyze(source: String, context_json: String) -> Result<JsValue, JsValue> {
    // Parse the context from the provided JSON string.
    let ctx = Converter::parse_context(&context_json)?;

    // Perform the analysis and collect the diagnostics.
    let result: AnalyzeResult = match analyzer::analyze(&source) {
        Ok(mut output) => {
            // Analyze the expression and append the diagnostics.
            let (_, diags) = analyzer::semantic::analyze_expr(&output.expr, &ctx);
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

/// Complete code at a given cursor position and return the result in a serialized `JsValue`.
///
/// # Arguments:
/// * `source` - The source string in which to complete the code.
/// * `cursor` - The cursor position (UTF-16 offset) where completion is requested.
/// * `context_json` - A JSON string representing the context for completion.
///
/// # Returns:
/// A `JsValue` representing the serialized completion output, or an error if the serialization fails.
#[wasm_bindgen]
pub fn complete(source: String, cursor: usize, context_json: String) -> Result<JsValue, JsValue> {
    // Convert the cursor position from UTF-16 to byte offset.
    let cursor_byte = Converter::cursor_utf16_to_byte(&source, cursor);

    // Parse the context from the provided JSON string.
    let ctx = Converter::parse_context(&context_json)?;

    // Perform the completion operation.
    let output = analyzer::complete(&source, cursor_byte, &ctx);

    // Convert the completion output to the desired DTO format.
    let out = Converter::completion_output_view(&source, &output);
    serde_wasm_bindgen::to_value(&out).map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

/// Convert a byte offset to a line and column, and return the result as a `JsValue`.
///
/// # Arguments:
/// * `source` - The source string to calculate the line and column for.
/// * `pos` - The byte position for which to calculate the line and column.
///
/// # Returns:
/// A `JsValue` representing the line and column view at the given byte position.
#[wasm_bindgen]
pub fn pos_to_line_col(source: String, pos: u32) -> JsValue {
    // Convert the position to a line and column view.
    let out: LineColView = Converter::pos_to_line_col_view(&source, pos);

    serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
}
