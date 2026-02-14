//! WASM DTOs (v1).
//!
//! JS-facing types returned by `analyzer_wasm`.
//! Spans and offsets use UTF-16 code units and are half-open `[start, end)`.

use analyzer::analysis::Ty as AnalyzerTy;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "PascalCase")]
pub enum Ty {
    Number,
    String,
    Boolean,
    Date,
    List(Box<Ty>),
}

impl From<Ty> for AnalyzerTy {
    fn from(ty: Ty) -> AnalyzerTy {
        match ty {
            Ty::Number => AnalyzerTy::Number,
            Ty::String => AnalyzerTy::String,
            Ty::Boolean => AnalyzerTy::Boolean,
            Ty::Date => AnalyzerTy::Date,
            Ty::List(ty) => AnalyzerTy::List(Box::new((*ty).into())),
        }
    }
}

/// A property available to `prop("Name")` calls and to editor completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Property {
    /// Canonical property name as referenced by `prop("...")`.
    pub name: String,
    #[serde(rename = "type")]
    /// Declared property type.
    pub ty: Ty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AnalyzerConfig {
    #[serde(default)]
    pub properties: Vec<Property>,
    pub preferred_limit: Option<usize>,
}

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
pub enum DiagnosticKind {
    #[serde(rename = "error")]
    Error,
}

/// A text edit in UTF-16 coordinates.
#[derive(Serialize, Deserialize, TS)]
pub struct TextEdit {
    /// Replace range in the original document (UTF-16, half-open).
    pub range: Span,
    /// Inserted verbatim.
    pub new_text: String,
}

/// A single diagnostic-attached code action.
#[derive(Serialize, TS)]
pub struct CodeAction {
    pub title: String,
    /// Edits are in original-document coordinates (UTF-16).
    pub edits: Vec<TextEdit>,
}

/// A diagnostic message tied to a source span.
#[derive(Serialize, TS)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    /// Location in the source text (UTF-16 span).
    pub span: Span,
    /// 1-based line number derived from source byte offsets.
    pub line: usize,
    /// 1-based column number as Unicode scalar (`char`) count.
    pub col: usize,
    /// Diagnostic-level code actions.
    pub actions: Vec<CodeAction>,
}

/// A token view for editor tooling.
#[derive(Serialize, TS)]
pub struct Token {
    pub kind: String,
    pub text: String,
    /// Location in the source text (UTF-16 span).
    pub span: Span,
}

/// Result payload returned from the `analyze` WASM export.
#[derive(Serialize, TS)]
pub struct AnalyzeResult {
    pub diagnostics: Vec<Diagnostic>,
    pub tokens: Vec<Token>,
    /// Inferred root expression type rendered for UI (e.g. `"number | string"`).
    ///
    /// Never nullable. Unknown/failed inference is represented as `"unknown"`.
    pub output_type: String,
}

/// Result payload for `format` and `apply_edits`.
#[derive(Serialize, TS)]
pub struct ApplyResult {
    pub source: String,
    /// Cursor position in the updated document (UTF-16).
    pub cursor: u32,
}

/// Signature help for a call expression.
#[derive(Serialize, TS)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureItem>,
    pub active_signature: usize,
    pub active_parameter: usize,
}

/// A single signature in signature help.
#[derive(Serialize, TS)]
pub struct SignatureItem {
    pub segments: Vec<DisplaySegment>,
}

#[derive(Serialize, TS)]
#[serde(tag = "kind", rename_all = "PascalCase")]
pub enum DisplaySegment {
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
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub insert_text: String,
    /// Primary edit to apply in the original document (UTF-16), if available.
    pub primary_edit: Option<TextEdit>,
    /// Cursor position in the updated document after applying edits (UTF-16).
    pub cursor: Option<u32>,
    /// Additional edits to apply in the original document (UTF-16).
    pub additional_edits: Vec<TextEdit>,
    pub detail: Option<String>,
    pub is_disabled: bool,
    pub disabled_reason: Option<String>,
}

/// Completion payload returned from `ide_help` (without signature help).
#[derive(Serialize, TS)]
pub struct CompletionResult {
    pub items: Vec<CompletionItem>,
    /// Replace range in the original document (UTF-16).
    pub replace: Span,
    pub preferred_indices: Vec<usize>,
}

/// Help result payload returned from the `ide_help` WASM export.
#[derive(Serialize, TS)]
pub struct HelpResult {
    pub completion: CompletionResult,
    pub signature_help: Option<SignatureHelp>,
}
