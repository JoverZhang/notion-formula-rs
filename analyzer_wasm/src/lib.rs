pub mod dto;
mod offsets;
mod span;
mod text_edit;
mod view;

use analyzer::SourceMap;
use analyzer::semantic::{Context, builtins_functions};
use wasm_bindgen::prelude::*;

use crate::dto::v1::{AnalyzeResult, LineColView};
use crate::offsets::utf16_offset_to_byte;
use crate::view::ViewCtx;

#[derive(Debug)]
enum ContextParseError {
    Empty,
    InvalidJson,
}

#[wasm_bindgen]
pub fn analyze(source: String, context_json: String) -> JsValue {
    let view = ViewCtx::new(&source);
    let (ctx, invalid_context) = match parse_context(&context_json) {
        Ok(ctx) => (ctx, false),
        Err(_) => (
            Context {
                properties: vec![],
                functions: builtins_functions(),
            },
            true,
        ),
    };
    let mut result: AnalyzeResult = match analyzer::analyze_with_context(&source, ctx) {
        Ok(output) => view.analyze_output(output),
        Err(diag) => view.analyze_error(&diag),
    };

    if invalid_context {
        result.diagnostics.push(view.invalid_context_diag());
    }

    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn complete(source: String, cursor: usize, context_json: String) -> JsValue {
    let cursor_byte = utf16_offset_to_byte(&source, cursor);
    let ctx = parse_context(&context_json).unwrap_or_else(|_| Context {
        properties: vec![],
        functions: builtins_functions(),
    });

    let view = ViewCtx::new(&source);
    let output = analyzer::complete_with_context(&source, cursor_byte, Some(&ctx));
    let out = view.completion_output(&output);
    serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn pos_to_line_col(source: String, pos: u32) -> JsValue {
    let byte = utf16_offset_to_byte(&source, pos as usize);
    let (line, col) = SourceMap::new(&source).line_col(byte as u32);
    let out = LineColView {
        line: line as u32,
        col: col as u32,
    };
    serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
}

fn parse_context(context_json: &str) -> Result<Context, ContextParseError> {
    let trimmed = context_json.trim();
    if trimmed.is_empty() {
        return Err(ContextParseError::Empty);
    }

    match serde_json::from_str::<Context>(trimmed) {
        Ok(mut ctx) => {
            ctx.functions = builtins_functions();
            Ok(ctx)
        }
        Err(_) => Err(ContextParseError::InvalidJson),
    }
}
