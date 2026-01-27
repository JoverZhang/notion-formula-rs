use serde::Serialize;
use ts_rs::TS;

/// JS/editor-facing span in UTF-16 code units.
///
/// Ranges are half-open `[start, end)`; `end` is exclusive.
#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionItemKind {
    Function,
    Builtin,
    Property,
    Operator,
}

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

#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKindView {
    #[serde(rename = "error")]
    Error,
}

#[derive(Serialize, TS)]
pub struct SpanView {
    pub range: Span,
}

#[derive(Serialize, TS, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineColView {
    pub line: u32,
    pub col: u32,
}

#[derive(Serialize, TS)]
pub struct DiagnosticView {
    pub kind: DiagnosticKindView,
    pub message: String,
    pub span: SpanView,
}

#[derive(Serialize, TS)]
pub struct TokenView {
    pub kind: String,
    pub text: String,
    pub span: SpanView,
}

#[derive(Serialize, TS)]
pub struct AnalyzeResult {
    pub diagnostics: Vec<DiagnosticView>,
    pub tokens: Vec<TokenView>,
    pub formatted: String,
}

#[derive(Serialize, TS)]
pub struct TextEditView {
    pub range: Span,
    pub new_text: String,
}

#[derive(Serialize, TS)]
pub struct SignatureHelpView {
    pub label: String,
    pub params: Vec<String>,
    pub active_param: usize,
}

#[derive(Serialize, TS)]
pub struct CompletionItemView {
    pub label: String,
    pub kind: CompletionItemKind,
    pub category: Option<FunctionCategoryView>,
    pub insert_text: String,
    pub primary_edit: Option<TextEditView>,
    pub cursor: Option<u32>,
    pub additional_edits: Vec<TextEditView>,
    pub detail: Option<String>,
    pub is_disabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Serialize, TS)]
pub struct CompletionOutputView {
    pub items: Vec<CompletionItemView>,
    pub replace: Span,
    pub signature_help: Option<SignatureHelpView>,
    pub preferred_indices: Vec<usize>,
}
