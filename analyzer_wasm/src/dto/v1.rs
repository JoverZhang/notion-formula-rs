//! WASM DTOs (v1).
//!
//! JS-facing types returned by `analyzer_wasm`.
//! Spans and offsets use UTF-16 code units and are half-open `[start, end)`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A span in UTF-16 code units (half-open `[start, end)`).
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// Start offset in UTF-16 code units.
    pub start: u32,
    /// End offset in UTF-16 code units (exclusive).
    pub end: u32,
}

/// Completion item kind.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionItemKind {
    FunctionGeneral,
    FunctionText,
    FunctionNumber,
    FunctionDate,
    FunctionPeople,
    FunctionList,
    FunctionSpecial,
    Builtin,
    Property,
    Operator,
}

/// Diagnostic severity/kind.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKindView {
    #[serde(rename = "error")]
    Error,
}

/// Wrapper around a [`Span`].
#[derive(Serialize, TS)]
pub struct SpanView {
    pub range: Span,
}

/// A text edit in UTF-16 coordinates.
#[derive(Serialize, Deserialize, TS)]
pub struct TextEditView {
    /// Replace range in the original document (UTF-16, half-open).
    pub range: Span,
    /// Inserted verbatim.
    pub new_text: String,
}

/// A single diagnostic-attached code action.
#[derive(Serialize, TS)]
pub struct CodeActionView {
    pub title: String,
    /// Edits are in original-document coordinates (UTF-16).
    pub edits: Vec<TextEditView>,
}

/// A diagnostic message tied to a source span.
#[derive(Serialize, TS)]
pub struct DiagnosticView {
    pub kind: DiagnosticKindView,
    pub message: String,
    /// Location in the source text (UTF-16 span).
    pub span: SpanView,
    /// 1-based line number derived from source byte offsets.
    pub line: usize,
    /// 1-based column number as Unicode scalar (`char`) count.
    pub col: usize,
    /// Diagnostic-level code actions.
    pub actions: Vec<CodeActionView>,
}

/// A token view for editor tooling.
#[derive(Serialize, TS)]
pub struct TokenView {
    pub kind: String,
    pub text: String,
    /// Location in the source text (UTF-16 span).
    pub span: SpanView,
}

/// Result payload returned from the `analyze` WASM export.
#[derive(Serialize, TS)]
pub struct AnalyzeResult {
    pub diagnostics: Vec<DiagnosticView>,
    pub tokens: Vec<TokenView>,
    /// Inferred root expression type rendered for UI (e.g. `"number | string"`).
    ///
    /// Never nullable. Unknown/failed inference is represented as `"unknown"`.
    pub output_type: String,
}

/// Result payload for `format` and `apply_edits`.
#[derive(Serialize, TS)]
pub struct ApplyResultView {
    pub source: String,
    /// Cursor position in the updated document (UTF-16).
    pub cursor: u32,
}

/// Signature help for a call expression.
#[derive(Serialize, TS)]
pub struct SignatureHelpView {
    pub signatures: Vec<SignatureItemView>,
    pub active_signature: usize,
    pub active_parameter: usize,
}

/// A single signature in signature help.
#[derive(Serialize, TS)]
pub struct SignatureItemView {
    pub segments: Vec<DisplaySegmentView>,
}

#[derive(Serialize, TS)]
#[serde(tag = "kind", rename_all = "PascalCase")]
pub enum DisplaySegmentView {
    Name {
        text: String,
    },
    Punct {
        text: String,
    },
    Separator {
        text: String,
    },
    Ellipsis,
    Arrow {
        text: String,
    },
    Param {
        name: String,
        ty: String,
        param_index: Option<u32>,
    },
    ReturnType {
        text: String,
    },
}

/// A single completion item.
#[derive(Serialize, TS)]
pub struct CompletionItemView {
    pub label: String,
    pub kind: CompletionItemKind,
    pub insert_text: String,
    /// Primary edit to apply in the original document (UTF-16), if available.
    pub primary_edit: Option<TextEditView>,
    /// Cursor position in the updated document after applying edits (UTF-16).
    pub cursor: Option<u32>,
    /// Additional edits to apply in the original document (UTF-16).
    pub additional_edits: Vec<TextEditView>,
    pub detail: Option<String>,
    pub is_disabled: bool,
    pub disabled_reason: Option<String>,
}

/// Completion result payload returned from the `complete` WASM export.
#[derive(Serialize, TS)]
pub struct CompletionOutputView {
    pub items: Vec<CompletionItemView>,
    /// Replace range in the original document (UTF-16).
    pub replace: Span,
    pub signature_help: Option<SignatureHelpView>,
    pub preferred_indices: Vec<usize>,
}
