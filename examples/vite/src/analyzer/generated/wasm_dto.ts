/* eslint-disable */
/* prettier-ignore */
// AUTO-GENERATED: `cargo run -p analyzer_wasm --bin export_ts`

export type SpanView = { start: number, end: number, line: number, col: number, };

export type DiagnosticKindView = "error";

export type DiagnosticView = { kind: DiagnosticKindView, message: string, span: SpanView, };

export type TokenView = { kind: string, text: string, span: SpanView, };

export type AnalyzeResult = { diagnostics: Array<DiagnosticView>, tokens: Array<TokenView>, formatted: string, };

export type SimpleSpanView = { start: number, end: number, };

export type TextEditView = { range: SimpleSpanView, new_text: string, };

export type SignatureHelpView = { label: string, params: Array<string>, active_param: number, };

export type CompletionItemKind = "Function" | "Builtin" | "Property" | "Operator";

export type CompletionItemView = { label: string, kind: CompletionItemKind, insert_text: string, primary_edit: TextEditView | null, cursor: number | null, additional_edits: Array<TextEditView>, detail: string | null, is_disabled: boolean, disabled_reason: string | null, };

export type CompletionOutputView = { items: Array<CompletionItemView>, replace: SimpleSpanView, signature_help: SignatureHelpView | null, };

