//! WASM DTOs (v1).
//!
//! JS-facing types returned by `analyzer_wasm`.
//! Spans and offsets use UTF-16 code units and are half-open `[start, end)`.

use serde::Serialize;
use ts_rs::TS;

/// A span in UTF-16 code units (half-open `[start, end)`).
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// Start offset in UTF-16 code units.
    pub start: u32,
    /// End offset in UTF-16 code units (exclusive).
    pub end: u32,
}

/// Completion item kind.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionItemKind {
    Function,
    Builtin,
    Property,
    Operator,
}

/// Function category.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionCategoryView {
    General,
    Text,
    Number,
    Date,
    People,
    List,
    Special,
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

/// 1-based line/column location in the source text.
///
/// This is computed by `pos_to_line_col` using core `SourceMap` rules.
/// It is not a UTF-16 offset pair.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineColView {
    pub line: u32,
    /// 1-based column from `SourceMap::line_col`.
    ///
    /// This is a Rust `char` count (Unicode scalar values). It is not UTF-16.
    pub col: u32,
}

/// A diagnostic message tied to a source span.
#[derive(Serialize, TS)]
pub struct DiagnosticView {
    pub kind: DiagnosticKindView,
    pub message: String,
    /// Location in the source text (UTF-16 span).
    pub span: SpanView,
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
    pub formatted: String,
}

/// A text edit in UTF-16 coordinates.
#[derive(Serialize, TS)]
pub struct TextEditView {
    /// Replace range in the original document (UTF-16, half-open).
    pub range: Span,
    /// Inserted verbatim.
    pub new_text: String,
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
    pub category: Option<FunctionCategoryView>,
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
