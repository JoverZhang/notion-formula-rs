use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, TS)]
pub struct SpanView {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

#[derive(Serialize, TS)]
pub struct DiagnosticView {
    pub kind: String,
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
pub struct SimpleSpanView {
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, TS)]
pub struct TextEditView {
    pub range: SimpleSpanView,
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
    pub kind: String,
    pub insert_text: String,
    pub primary_edit: Option<TextEditView>,
    pub cursor: Option<usize>,
    pub additional_edits: Vec<TextEditView>,
    pub detail: Option<String>,
    pub is_disabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Serialize, TS)]
pub struct CompletionOutputView {
    pub items: Vec<CompletionItemView>,
    pub replace: SimpleSpanView,
    pub signature_help: Option<SignatureHelpView>,
}
