/* eslint-disable */
/* prettier-ignore */
// AUTO-GENERATED: `cargo run -p analyzer_wasm --bin export_ts`

export type Span = { 
/**
 * Start offset in UTF-16 code units.
 */
start: number, 
/**
 * End offset in UTF-16 code units (exclusive).
 */
end: number, };

export type SpanView = { range: Span, };

export type TextEditView = { 
/**
 * Replace range in the original document (UTF-16, half-open).
 */
range: Span, 
/**
 * Inserted verbatim.
 */
new_text: string, };

export type CodeActionView = { title: string, 
/**
 * Edits are in original-document coordinates (UTF-16).
 */
edits: Array<TextEditView>, };

export type DiagnosticKindView = "error";

export type DiagnosticView = { kind: DiagnosticKindView, message: string, 
/**
 * Location in the source text (UTF-16 span).
 */
span: SpanView, 
/**
 * 1-based line number derived from source byte offsets.
 */
line: number, 
/**
 * 1-based column number as Unicode scalar (`char`) count.
 */
col: number, 
/**
 * Diagnostic-level code actions.
 */
actions: Array<CodeActionView>, };

export type TokenView = { kind: string, text: string, 
/**
 * Location in the source text (UTF-16 span).
 */
span: SpanView, };

export type AnalyzeResult = { diagnostics: Array<DiagnosticView>, tokens: Array<TokenView>, 
/**
 * Inferred root expression type rendered for UI (e.g. `"number | string"`).
 *
 * Never nullable. Unknown/failed inference is represented as `"unknown"`.
 */
output_type: string, };

export type ApplyResultView = { source: string, 
/**
 * Cursor position in the updated document (UTF-16).
 */
cursor: number, };

export type DisplaySegmentView = { "kind": "Name", text: string, } | { "kind": "Punct", text: string, } | { "kind": "Separator", text: string, } | { "kind": "Ellipsis" } | { "kind": "Arrow", text: string, } | { "kind": "Param", name: string, ty: string, param_index: number | null, } | { "kind": "ReturnType", text: string, };

export type SignatureItemView = { segments: Array<DisplaySegmentView>, };

export type SignatureHelpView = { signatures: Array<SignatureItemView>, active_signature: number, active_parameter: number, };

export type CompletionItemKind = "FunctionGeneral" | "FunctionText" | "FunctionNumber" | "FunctionDate" | "FunctionPeople" | "FunctionList" | "FunctionSpecial" | "Builtin" | "Property" | "Operator";

export type CompletionItemView = { label: string, kind: CompletionItemKind, insert_text: string, 
/**
 * Primary edit to apply in the original document (UTF-16), if available.
 */
primary_edit: TextEditView | null, 
/**
 * Cursor position in the updated document after applying edits (UTF-16).
 */
cursor: number | null, 
/**
 * Additional edits to apply in the original document (UTF-16).
 */
additional_edits: Array<TextEditView>, detail: string | null, is_disabled: boolean, disabled_reason: string | null, };

export type CompletionOutputView = { items: Array<CompletionItemView>, 
/**
 * Replace range in the original document (UTF-16).
 */
replace: Span, signature_help: SignatureHelpView | null, preferred_indices: Array<number>, };

