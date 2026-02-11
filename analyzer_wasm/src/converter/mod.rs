//! Conversion utilities for the WASM/JS boundary.
//!
//! This module is intentionally stateless and centralizes:
//! - Input conversion (e.g. parsing context JSON).
//! - UTF-16 ↔ byte offset bridging for editor-facing positions.
//! - DTO conversion (internal analyzer types → `dto::v1::*` views).

mod analyze;
mod completion;
mod context;
mod shared;

use wasm_bindgen::prelude::JsValue;

use crate::dto::v1::{AnalyzeResult, CompletionOutputView};
use crate::offsets::utf16_offset_to_byte;

pub use context::ParsedContext;

pub struct Converter;

impl Converter {
    /// Parse the JS-provided context JSON into an analyzer `Context`.
    pub fn parse_context(context_json: &str) -> Result<ParsedContext, JsValue> {
        context::parse_context(context_json)
    }

    /// Convert a UTF-16 cursor offset (CodeMirror) to a byte offset (Rust strings).
    pub fn cursor_utf16_to_byte(source: &str, cursor_utf16: usize) -> usize {
        utf16_offset_to_byte(source, cursor_utf16)
    }

    pub fn analyze_output(
        source: &str,
        output: analyzer::ParseOutput,
        output_type: String,
    ) -> AnalyzeResult {
        analyze::analyze_output(source, output, output_type)
    }

    pub fn analyze_error(source: &str, diag: &analyzer::Diagnostic) -> AnalyzeResult {
        analyze::analyze_error(source, diag)
    }

    pub fn completion_output_view(
        source: &str,
        output: &analyzer::CompletionOutput,
    ) -> CompletionOutputView {
        completion::completion_output_view(source, output)
    }
}
