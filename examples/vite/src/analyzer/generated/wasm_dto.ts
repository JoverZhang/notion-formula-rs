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

export type LineColView = { line: number, 
/**
 * 1-based column from `SourceMap::line_col`.
 *
 * This is a Rust `char` count (Unicode scalar values). It is not UTF-16.
 */
col: number, };

export type DiagnosticKindView = "error";

export type DiagnosticView = { kind: DiagnosticKindView, message: string, 
/**
 * Location in the source text (UTF-16 span).
 */
span: SpanView, };

export type TokenView = { kind: string, text: string, 
/**
 * Location in the source text (UTF-16 span).
 */
span: SpanView, };

export type AnalyzeResult = { diagnostics: Array<DiagnosticView>, tokens: Array<TokenView>, formatted: string, };

export type TextEditView = { 
/**
 * Replace range in the original document (UTF-16, half-open).
 */
range: Span, 
/**
 * Inserted verbatim.
 */
new_text: string, };

export type DisplaySegmentKindView = "Name" | "Punct" | "ParamName" | "Type" | "Separator" | "Ellipsis" | "Arrow" | "ReturnType";

export type DisplaySegmentView = { kind: DisplaySegmentKindView, text: string, param_index: number | null, };

export type SignatureHelpSignatureView = { segments: Array<DisplaySegmentView>, };

export type SignatureHelpView = { signatures: Array<SignatureHelpSignatureView>, active_signature: number, active_parameter: number, };

export type CompletionItemKind = "Function" | "Builtin" | "Property" | "Operator";

export type FunctionCategoryView = "General" | "Text" | "Number" | "Date" | "People" | "List" | "Special";

export type CompletionItemView = { label: string, kind: CompletionItemKind, category: FunctionCategoryView | null, insert_text: string, 
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

