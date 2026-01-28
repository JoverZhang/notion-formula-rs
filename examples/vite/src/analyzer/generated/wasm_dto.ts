/* eslint-disable */
/* prettier-ignore */
// AUTO-GENERATED: `cargo run -p analyzer_wasm --bin export_ts`

export type Span = { start: number, end: number, };

export type SpanView = { range: Span, };

export type LineColView = { line: number, col: number, };

export type DiagnosticKindView = "error";

export type DiagnosticView = { kind: DiagnosticKindView, message: string, span: SpanView, };

export type TokenView = { kind: string, text: string, span: SpanView, };

export type AnalyzeResult = { diagnostics: Array<DiagnosticView>, tokens: Array<TokenView>, formatted: string, };

export type TextEditView = { range: Span, new_text: string, };

export type SignatureHelpView = { receiver: string | null, label: string, params: Array<string>, active_param: number, };

export type CompletionItemKind = "Function" | "Builtin" | "Property" | "Operator";

export type FunctionCategoryView = "General" | "Text" | "Number" | "Date" | "People" | "List" | "Special";

export type CompletionItemView = { label: string, kind: CompletionItemKind, category: FunctionCategoryView | null, insert_text: string, primary_edit: TextEditView | null, cursor: number | null, additional_edits: Array<TextEditView>, detail: string | null, is_disabled: boolean, disabled_reason: string | null, };

export type CompletionOutputView = { items: Array<CompletionItemView>, replace: Span, signature_help: SignatureHelpView | null, preferred_indices: Array<number>, };

