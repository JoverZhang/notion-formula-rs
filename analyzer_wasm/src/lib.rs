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

#[wasm_bindgen]
pub fn analyze(source: String, context_json: Option<String>) -> JsValue {
    let view = ViewCtx::new(&source);
    let result: AnalyzeResult = match context_json.as_deref().map(str::trim) {
        None | Some("") => match analyzer::analyze_with_context(
            &source,
            Context {
                properties: vec![],
                functions: builtins_functions(),
            },
        ) {
            Ok(output) => view.analyze_output(output),
            Err(diag) => view.analyze_error(&diag),
        },
        Some(context_json) => match serde_json::from_str::<Context>(context_json) {
            Ok(mut ctx) => {
                ctx.functions = builtins_functions();
                match analyzer::analyze_with_context(&source, ctx) {
                    Ok(output) => view.analyze_output(output),
                    Err(diag) => view.analyze_error(&diag),
                }
            }
            Err(_) => {
                let mut result = match analyzer::analyze(&source) {
                    Ok(output) => view.analyze_output(output),
                    Err(diag) => view.analyze_error(&diag),
                };

                result.diagnostics.push(view.invalid_context_diag());
                result
            }
        },
    };

    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn complete(source: String, cursor_utf16: usize, context_json: Option<String>) -> JsValue {
    let cursor_byte = utf16_offset_to_byte(&source, cursor_utf16);
    let mut ctx = parse_context(context_json.as_deref());
    ctx.as_mut().unwrap().functions = builtins_functions();

    let view = ViewCtx::new(&source);
    let output = analyzer::complete_with_context(&source, cursor_byte, ctx.as_ref());
    let out = view.completion_output(&output);
    serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn utf16_pos_to_line_col(source: String, pos_utf16: u32) -> JsValue {
    let byte = utf16_offset_to_byte(&source, pos_utf16 as usize);
    let (line, col) = SourceMap::new(&source).line_col(byte as u32);
    let out = LineColView {
        line: line as u32,
        col: col as u32,
    };
    serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
}

fn parse_context(context_json: Option<&str>) -> Option<Context> {
    match context_json.map(str::trim) {
        None | Some("") => None,
        Some(json) => serde_json::from_str::<Context>(json).ok(),
    }
}
