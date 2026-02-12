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

export type TextEdit = { 
/**
 * Replace range in the original document (UTF-16, half-open).
 */
range: Span, 
/**
 * Inserted verbatim.
 */
new_text: string, };

export type CodeAction = { title: string, 
/**
 * Edits are in original-document coordinates (UTF-16).
 */
edits: Array<TextEdit>, };

export type DiagnosticKind = "error";

export type Diagnostic = { kind: DiagnosticKind, message: string, 
/**
 * Location in the source text (UTF-16 span).
 */
span: Span, 
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
actions: Array<CodeAction>, };

export type Token = { kind: string, text: string, 
/**
 * Location in the source text (UTF-16 span).
 */
span: Span, };

export type AnalyzeResult = { diagnostics: Array<Diagnostic>, tokens: Array<Token>, 
/**
 * Inferred root expression type rendered for UI (e.g. `"number | string"`).
 *
 * Never nullable. Unknown/failed inference is represented as `"unknown"`.
 */
output_type: string, };

export type ApplyResult = { source: string, 
/**
 * Cursor position in the updated document (UTF-16).
 */
cursor: number, };

export type DisplaySegment = { "kind": "Name", text: string, } | { "kind": "Punct", text: string, } | { "kind": "Separator", text: string, } | { "kind": "Ellipsis" } | { "kind": "Arrow", text: string, } | { "kind": "Param", name: string, ty: string, param_index: number | null, } | { "kind": "ReturnType", text: string, };

export type SignatureItem = { segments: Array<DisplaySegment>, };

export type SignatureHelp = { signatures: Array<SignatureItem>, active_signature: number, active_parameter: number, };

export type CompletionItemKind = "FunctionGeneral" | "FunctionText" | "FunctionNumber" | "FunctionDate" | "FunctionPeople" | "FunctionList" | "FunctionSpecial" | "Builtin" | "Property" | "Operator";

export type CompletionItem = { label: string, kind: CompletionItemKind, insert_text: string, 
/**
 * Primary edit to apply in the original document (UTF-16), if available.
 */
primary_edit: TextEdit | null, 
/**
 * Cursor position in the updated document after applying edits (UTF-16).
 */
cursor: number | null, 
/**
 * Additional edits to apply in the original document (UTF-16).
 */
additional_edits: Array<TextEdit>, detail: string | null, is_disabled: boolean, disabled_reason: string | null, };

export type CompletionOutput = { items: Array<CompletionItem>, 
/**
 * Replace range in the original document (UTF-16).
 */
replace: Span, signature_help: SignatureHelp | null, preferred_indices: Array<number>, };

