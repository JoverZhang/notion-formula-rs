/* eslint-disable */
/* prettier-ignore */
// AUTO-GENERATED: `cargo run -p analyzer_wasm --bin export_ts`

export type PropertyId = string;

export type RowId = string;

export type DraftVersion = bigint;

export type DiagnosticId = string;

export type TextOffset = number;

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

export type Token = { kind: string, text: string, 
/**
 * Location in the source text (UTF-16 span).
 */
span: Span, };

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

export type CompletionResult = { items: Array<CompletionItem>, replace: Span, preferred_indices: Array<number>, };

export type ValueType = "Number" | "String" | "Boolean" | "Date" | "Unknown" | { "List": ValueType } | { "Union": Array<ValueType> };

export type FormulaSchema = { properties: Array<PropertyDefinition>, };

export type PropertyDefinition = { "Input": { id: string, ty: ValueType, } } | { "Formula": FormulaDefinition };

export type FormulaDefinition = { id: string, expression: string, };

export type FormulaEngineState = "AllReady" | { "NotAllReady": { cycle_path: Array<string>, } };

export type PropertyState = { "Input": { id: string, ty: ValueType, } } | { "Formula": FormulaState };

export type FormulaState = { definition: FormulaDefinition, status: FormulaStatus, };

export type FormulaStatus = { "Ready": { output_type: ValueType, } } | "NotReady";

export type FormulaEngineChangeResult = { affected_formulas: Array<string>, };

export type Value = { "Number": number } | { "String": string } | { "Boolean": boolean } | { "Date": bigint } | { "List": Array<Value | null> };

export type ColumnData<T> = { values: Array<T>, validity: Array<boolean>, };

export type Column = { "Number": ColumnData<number> } | { "String": ColumnData<string> } | { "Boolean": ColumnData<boolean> } | { "Date": ColumnData<bigint> } | { "List": ColumnData<Array<Value | null>> } | { "Union": ColumnData<Value> };

export type ColumnKind = "Number" | "String" | "Boolean" | "Date" | "List" | "Union";

export type RuntimeContext = { now: bigint, time_zone: string, };

export type EvaluateInput = { row_ids: Array<string>, columns: Map<PropertyId, Column>, runtime: RuntimeContext, formula_ids: Array<string>, };

export type EvaluateResult = { formulas: Map<PropertyId, { Ok: FormulaOutput } | { Err: FormulaEvaluationError }>, };

export type FormulaOutput = { output_type: ValueType, column: Column, errors: Array<RowError>, };

export type RowError = { row_index: number, origin_formula_id: string, error: RuntimeError, };

export type RuntimeError = { "InvalidValueType": { expected: ValueType, actual: ValueType, } } | { "InvalidValue": { actual: Value, constraint: string, } } | { "InvalidRegex": { pattern: string, detail: string, } } | { "InvalidDateText": { text: string, } } | "DateOutOfRange";

export type FormulaEvaluationError = "NotReady";

export type FormulaEngineInitError = "EmptyId" | { "DuplicateId": string };

export type EngineChangeError = "EmptyId";

export type CreateDraftError = "EmptyId";

export type EvaluateInputError = { "InvalidNow": { now: bigint, } } | { "InvalidTimeZone": { time_zone: string, } } | { "EmptyRowId": { row_index: number, } } | { "DuplicateRowId": { id: string, } } | "EmptyFormulaIds" | { "InvalidFormulaId": { id: string, } } | { "DuplicateFormulaId": { id: string, } } | { "MissingInputs": { ids: Array<string>, } } | { "UnexpectedInputs": { ids: Array<string>, } } | { "InvalidColumnType": { id: string, expected: ColumnKind, actual: ColumnKind, } } | { "InvalidColumnLength": { id: string, expected: number, values_len: number, validity_len: number, } } | { "InvalidValueType": { id: string, row_index: number, element_path: Array<number>, expected: ValueType, actual: ValueType, } };

export type CompletionConfig = { preferred_limit: number, };

export type FormulaDraftState = { version: bigint, definition: FormulaDefinition, output_type: ValueType, diagnostics: Array<ExpressionDiagnostic>, tokens: Array<Token>, };

export type ExpressionDiagnostic = { id: string, span: Span, message: string, };

export type CursorHelp = { base_version: bigint, completion: CompletionResult, signature_help: SignatureHelp | null, };

export type FormulaEdit = { base_version: bigint, edits: Array<TextEdit>, };

export type ExpressionUpdate = { "Replace": string } | { "Edits": { edit: FormulaEdit, cursor: number, } };

export type QuickFix = { title: string, edit: FormulaEdit, };

export type UpdateExpressionResult = { state: FormulaDraftState, cursor: number, };

export type UpdateExpressionError = "VersionMismatch" | "InvalidCursor" | "InvalidEditRange" | "OverlappingEdits";

export type InvalidDtoPayload = { operation: string, };

export type DraftClosedPayload = { handle: number, };

export type ActiveDraftsPayload = { count: number, };

export type EngineInitPayload = { error: FormulaEngineInitError, };

export type EngineChangePayload = { error: EngineChangeError, };

export type CreateDraftPayload = { error: CreateDraftError, };

export type EvaluateInputPayload = { error: EvaluateInputError, };

export type UpdateExpressionPayload = { error: UpdateExpressionError, };

export type FormulaClientErrorData = { "code": "INVALID_DTO", message: string, payload: InvalidDtoPayload, } | { "code": "ENGINE_CLOSED", message: string, payload: null, } | { "code": "DRAFT_CLOSED", message: string, payload: DraftClosedPayload, } | { "code": "ACTIVE_DRAFTS", message: string, payload: ActiveDraftsPayload, } | { "code": "ENGINE_INIT", message: string, payload: EngineInitPayload, } | { "code": "ENGINE_CHANGE", message: string, payload: EngineChangePayload, } | { "code": "CREATE_DRAFT", message: string, payload: CreateDraftPayload, } | { "code": "EVALUATE_INPUT", message: string, payload: EvaluateInputPayload, } | { "code": "UPDATE_EXPRESSION", message: string, payload: UpdateExpressionPayload, } | { "code": "FORMAT_ERROR", message: string, payload: null, } | { "code": "SERIALIZE_ERROR", message: string, payload: null, } | { "code": "WORKER_FAILURE", message: string, payload: null, } | { "code": "INVALID_RESPONSE", message: string, payload: null, } | { "code": "INVALID_REQUEST", message: string, payload: null, } | { "code": "INITIALIZATION_ERROR", message: string, payload: null, };

