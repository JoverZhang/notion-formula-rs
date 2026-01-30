//! Completion + signature help for editors.
//! All coordinates are UTF-8 byte offsets into the input `text`.
//! Spans are half-open ranges `[start, end)`.

use crate::lexer::Span;
use crate::semantic;

mod items;
mod matchers;
mod pipeline;
mod position;
mod rank;
mod signature;

/// Default for `CompletionConfig.preferred_limit`.
pub const DEFAULT_PREFERRED_LIMIT: usize = 5;

/// Configuration knobs for `complete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionConfig {
    /// Max length of `CompletionOutput.preferred_indices` (0 disables it).
    pub preferred_limit: usize,
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            preferred_limit: DEFAULT_PREFERRED_LIMIT,
        }
    }
}

/// Result of a completion query at a byte cursor.
///
/// `replace` and all edit ranges are UTF-8 byte spans, half-open `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionOutput {
    pub items: Vec<CompletionItem>,
    pub replace: Span,
    pub signature_help: Option<SignatureHelp>,
    /// Indices into `items` for the UI default selection.
    pub preferred_indices: Vec<usize>,
}

/// A single text edit in byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Span,
    pub new_text: String,
}

/// One completion candidate for an editor UI.
///
/// If `cursor` is set, it is a desired byte offset in the updated document after applying
/// the primary edit.
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

/// High-level bucket for UI grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Builtin,
    Property,
    Operator,
}

/// Extra metadata used for cursor placement and type ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionData {
    Function { name: String },
    PropExpr { property_name: String },
    PostfixMethod { name: String },
}

/// Signature display for a call at the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub receiver: Option<String>,
    pub label: String,
    pub params: Vec<String>,
    pub active_param: usize,
}

/// Computes completion items and signature help at a cursor position.
///
/// `cursor` is a UTF-8 byte offset into `text`.
pub fn complete(
    text: &str,
    cursor: usize,
    ctx: Option<&semantic::Context>,
    config: CompletionConfig,
) -> CompletionOutput {
    pipeline::complete(text, cursor, ctx, config)
}
