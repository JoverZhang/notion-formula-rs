//! WASM entry points for the analyzer.
//!
//! Core code uses UTF-8 byte offsets. The JS boundary uses UTF-16 code unit offsets.
//! Spans are half-open `[start, end)`.
mod converter;
pub mod dto;
mod offsets;
mod span;

use analyzer::analysis::{Context, Property as AnalyzerProperty, builtins_functions};
use ide::CompletionConfig;
use js_sys::Error as JsError;
use js_sys::Object;
use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::converter::Converter;
use crate::dto::v1::{AnalyzeResult, AnalyzerConfig, ApplyResult, TextEdit as Utf16TextEdit};
use crate::offsets::{utf16_to_8_cursor, utf16_to_8_text_edits};

const DEFAULT_PREFERRED_LIMIT: usize = 5;

#[wasm_bindgen]
pub struct Analyzer {
    context: Context,
    preferred_limit: usize,
}

#[wasm_bindgen]
impl Analyzer {
    /// Create a new analyzer with the given config.
    ///
    /// @param config: [`AnalyzerConfig`]
    /// @returns [`Analyzer`]
    /// @throws [`String`] if the config is invalid
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<Self, String> {
        validate_config_keys(&config)?;
        let input: AnalyzerConfig = from_value(config, "Invalid analyzer config")?;
        Ok(Self {
            context: Context {
                properties: input
                    .properties
                    .into_iter()
                    .map(|p| AnalyzerProperty {
                        name: p.name,
                        ty: p.ty.into(),
                        disabled_reason: None,
                    })
                    .collect(),
                functions: builtins_functions(),
            },
            preferred_limit: input.preferred_limit.unwrap_or(DEFAULT_PREFERRED_LIMIT),
        })
    }

    pub fn analyze(&self, source: String) -> Result<JsValue, JsValue> {
        let result = analyzer::analyze(&source, &self.context);
        let out: AnalyzeResult = Converter::analyze_output(&source, result);
        to_value(&out)
    }

    pub fn format(&self, source: String, cursor_utf16: u32) -> Result<JsValue, JsValue> {
        let cursor_byte = utf16_to_8_cursor(&source, cursor_utf16).map_err(operation_err)? as u32;
        let output = ide::format(&source, cursor_byte).map_err(operation_err)?;
        to_value(&ApplyResult {
            cursor: Converter::utf8_to_16_offset(&output.source, output.cursor as usize),
            source: output.source,
        })
    }

    pub fn apply_edits(
        &self,
        source: String,
        edits: JsValue,
        cursor_utf16: u32,
    ) -> Result<JsValue, JsValue> {
        let text_edits: Vec<Utf16TextEdit> = serde_wasm_bindgen::from_value(edits)
            .map_err(|_| JsValue::from(JsError::new("Invalid edits")))?;
        let text_edits = utf16_to_8_text_edits(&source, text_edits).map_err(operation_err)?;
        let cursor = utf16_to_8_cursor(&source, cursor_utf16).map_err(operation_err)? as u32;

        let result = ide::apply_edits(&source, text_edits, cursor).map_err(operation_err)?;

        to_value(&ApplyResult {
            cursor: Converter::utf8_to_16_offset(&result.source, result.cursor as usize),
            source: result.source,
        })
    }

    pub fn help(&self, source: String, cursor_utf16: u32) -> Result<JsValue, JsValue> {
        let cursor = Converter::utf16_to_8_offset(&source, cursor_utf16 as usize);

        let output = ide::help(
            &source,
            cursor,
            &self.context,
            CompletionConfig {
                preferred_limit: self.preferred_limit,
            },
        );
        to_value(&Converter::help_output_view(&source, &output))
    }
}

fn to_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|_| JsValue::from(JsError::new("Serialize error")))
}

fn from_value<T: serde::de::DeserializeOwned>(
    value: JsValue,
    err: &'static str,
) -> Result<T, String> {
    serde_wasm_bindgen::from_value(value).map_err(|_| err.to_string())
}

fn operation_err(err: ide::IdeError) -> JsValue {
    JsValue::from(JsError::new(err.message()))
}

fn validate_config_keys(config: &JsValue) -> Result<(), String> {
    if !config.is_object() {
        return Err("Invalid analyzer config".to_string());
    }

    let object = config.unchecked_ref::<Object>();
    let keys = Object::keys(object);
    for i in 0..keys.length() {
        let Some(key) = keys.get(i).as_string() else {
            return Err("Invalid analyzer config".to_string());
        };
        match key.as_str() {
            "properties" | "preferred_limit" => {}
            _ => return Err("Invalid analyzer config".to_string()),
        }
    }

    Ok(())
}
