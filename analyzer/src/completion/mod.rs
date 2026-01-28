use crate::semantic;
use crate::token::Span;

mod items;
mod matchers;
mod pipeline;
mod position;
mod rank;
mod signature;

pub const DEFAULT_PREFERRED_LIMIT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionConfig {
    pub preferred_limit: usize,
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            preferred_limit: DEFAULT_PREFERRED_LIMIT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionOutput {
    pub items: Vec<CompletionItem>,
    pub replace: Span,
    pub signature_help: Option<SignatureHelp>,
    pub preferred_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Span,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub category: Option<semantic::FunctionCategory>,
    pub insert_text: String,
    pub primary_edit: Option<TextEdit>,
    pub cursor: Option<u32>,
    pub additional_edits: Vec<TextEdit>,
    pub detail: Option<String>,
    pub is_disabled: bool,
    pub disabled_reason: Option<String>,
    pub data: Option<CompletionData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Builtin,
    Property,
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionData {
    Function { name: String },
    PropExpr { property_name: String },
    PostfixMethod { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub receiver: Option<String>,
    pub label: String,
    pub params: Vec<String>,
    pub active_param: usize,
}

/// Compute completions using byte offsets for the cursor and replace span.
pub fn complete(
    text: &str,
    cursor: usize,
    ctx: Option<&semantic::Context>,
    config: CompletionConfig,
) -> CompletionOutput {
    pipeline::complete(text, cursor, ctx, config)
}
