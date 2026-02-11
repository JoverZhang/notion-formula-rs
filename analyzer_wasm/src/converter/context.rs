use analyzer::completion::{CompletionConfig, DEFAULT_PREFERRED_LIMIT};
use analyzer::semantic::{Context, Property, builtins_functions};
use js_sys::Error as JsError;
use serde::Deserialize;
use wasm_bindgen::prelude::JsValue;

pub struct ParsedContext {
    pub ctx: Context,
    pub completion: CompletionConfig,
}

pub fn parse_context(context_json: &str) -> Result<ParsedContext, JsValue> {
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CompletionInput {
        #[serde(default = "default_preferred_limit")]
        preferred_limit: usize,
    }

    impl Default for CompletionInput {
        fn default() -> Self {
            Self {
                preferred_limit: default_preferred_limit(),
            }
        }
    }

    fn default_preferred_limit() -> usize {
        DEFAULT_PREFERRED_LIMIT
    }

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ContextInput {
        #[serde(default)]
        properties: Vec<Property>,
        #[serde(default)]
        completion: CompletionInput,
    }

    fn invalid_context_json_error() -> JsValue {
        JsValue::from(JsError::new("Invalid context JSON"))
    }

    let trimmed = context_json.trim();
    if trimmed.is_empty() {
        return Err(invalid_context_json_error());
    }

    let input: ContextInput =
        serde_json::from_str(trimmed).map_err(|_| invalid_context_json_error())?;
    Ok(ParsedContext {
        ctx: Context {
            properties: input.properties,
            functions: builtins_functions(),
        },
        completion: CompletionConfig {
            preferred_limit: input.completion.preferred_limit,
        },
    })
}
