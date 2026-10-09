---
doc_id: specs.wasm-api
title: "WASM API and Worker"
language: en
source_language: en
counterpart: ./wasm-api.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
translation_review_model: gpt-6-astra
last_verified: 2026-10-09
---

# WASM API and Worker

[简体中文](wasm-api.zh-CN.md) · [Specification index](README.md)

The Worker clients use the [Engine](formula-engine.md) and [Draft](ide.md) contracts through lossless DTOs. Analysis, evaluation and editing remain in Rust.

## Engine and Draft clients

```ts spec-file=packages/notion-formula/src/client.h.ts
import type {
  CompletionConfig,
  CursorHelp,
  DiagnosticId,
  EvaluateInput,
  EvaluateResult,
  ExpressionUpdate,
  FormulaDefinition,
  FormulaDraftState,
  FormulaEdit,
  FormulaEngineChangeResult,
  FormulaEngineState,
  FormulaSchema,
  PropertyDefinition,
  PropertyId,
  PropertyState,
  QuickFix,
  UpdateExpressionResult,
} from "./generated/wasm_dto.js";
import type { FormulaWorker } from "./rpc.js";

// Pure, synchronous; no Worker/WASM initialization. Quote a valid Unicode value,
// escaping backslash, double quote, newline and tab; preserve other characters.
// a"b -> "a\"b"; empty -> "".
export declare function encodeFormulaString(value: string): string;

// Pure, synchronous; input is one complete String token.text from the lexer.
// Decode once: \n -> newline, \t -> tab; any other escape yields its next character.
// "a\"b" -> a"b; "" -> empty; "\q" -> q. Preserve Unicode and raw controls.
export declare function decodeFormulaString(literal: string): string;

// An Engine and all its Drafts share one FIFO queue; rejected calls do not stop it.
// Each request snapshots its arguments at enqueue; later mutations cannot change it.
// Non-cloneable arguments reject with INVALID_REQUEST at their FIFO position.
export interface FormulaEngineClient {
  getProperty(id: PropertyId): Promise<PropertyState | null>;
  getProperties(): Promise<PropertyState[]>;
  getState(): Promise<FormulaEngineState>;
  // Sorted, distinct transitive Input IDs. Ready targets have complete closures;
  // NotReady targets expose known references only. Invalid selections reject with EVALUATE_INPUT.
  requiredInputs(formulaIds: PropertyId[]): Promise<PropertyId[]>;
  // Rejects ACTIVE_DRAFTS until every Draft has been consumed or closed.
  upsert(property: PropertyDefinition): Promise<FormulaEngineChangeResult>;
  remove(id: PropertyId): Promise<FormulaEngineChangeResult | null>;
  // EVALUATE_INPUT rejects; formula and row errors remain in EvaluateResult.
  // Every row uses the caller's RuntimeContext; the Engine reads no system clock.
  evaluate(input: EvaluateInput): Promise<EvaluateResult>;
  createDraft(formula: FormulaDefinition): Promise<FormulaDraftClient>;
  // Rejects new calls immediately, drains queued calls, releases Drafts before
  // Engine, and terminates its Worker. Repeated close returns the same Promise.
  close(): Promise<void>;
}

export interface FormulaDraftClient {
  // Diagnostic IDs are opaque, stable for this version and scoped to this client.
  getState(): Promise<FormulaDraftState>;
  // UTF-16 cursor: floor inside a surrogate pair; clamp past the document end.
  help(cursor: number, config: CompletionConfig): Promise<CursorHelp>;
  // Foreign or stale diagnostic IDs return []; all queries still enter the FIFO.
  quickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]>;
  formatEdits(): Promise<FormulaEdit>;
  // Edits and cursor use the original source; returned cursor uses the new source.
  // Preserve the bigint base_version; stale edits reject with UPDATE_EXPRESSION.
  updateExpression(update: ExpressionUpdate): Promise<UpdateExpressionResult>;
  // Consumes this Draft without saving. Persist with Engine.upsert({ Formula: ... }).
  intoDefinition(): Promise<FormulaDefinition>;
  // Discards the Draft; idempotent, including after Engine.close().
  close(): Promise<void>;
}

export interface FormulaClientOptions {
  // Optional Worker injection; the client owns and terminates the returned Worker.
  workerFactory?: () => FormulaWorker;
}

// Implemented by createFormulaEngineClient in client.ts. Resolves after WASM and
// Engine initialization; initialization failure releases the Worker.
// Controlled rejections are FormulaClientError; error.data.code discriminates
// its typed payload. A Worker failure settles every pending call.
export type CreateFormulaEngineClient = (
  schema: FormulaSchema,
  options?: FormulaClientOptions,
) => Promise<FormulaEngineClient>;
```

### Lossless DTOs

Rust declarations generate the JavaScript types. These DTOs use structured cloning rather than JSON; enum encodings are those emitted by serde. Optional results and unit payloads are explicit `null`; `number` indices and lengths remain ordinary JavaScript numbers. Strings must contain valid Unicode, without unpaired surrogates.

```rust out=analyzer_wasm/src/dto/engine.h.rs
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
pub use super::v1::{CompletionItem, SignatureItem, Span, TextEdit, Token};

pub type PropertyId = String;
pub type RowId = String;
/// JavaScript bigint; numeric values are rejected on input.
pub type DraftVersion = u64;
/// Opaque native ID; the Worker client adds its own session scope.
pub type DiagnosticId = String;
/// Finite integer UTF-16 code units, 0..=4_294_967_295; ranges are half-open.
/// Surrogate-pair interiors floor to the scalar start before overlap checks.
pub type TextOffset = u32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ValueType { Number, String, Boolean, Date, Unknown, List(Box<ValueType>), Union(Vec<ValueType>) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct FormulaSchema { pub properties: Vec<PropertyDefinition> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum PropertyDefinition { Input { id: PropertyId, ty: ValueType }, Formula(FormulaDefinition) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct FormulaDefinition { pub id: PropertyId, pub expression: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum FormulaEngineState { AllReady, NotAllReady { cycle_path: Vec<PropertyId> } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum PropertyState { Input { id: PropertyId, ty: ValueType }, Formula(FormulaState) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct FormulaState { pub definition: FormulaDefinition, pub status: FormulaStatus }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum FormulaStatus { Ready { output_type: ValueType }, NotReady }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct FormulaEngineChangeResult { pub affected_formulas: Vec<PropertyId> }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// Lossless date metadata; both endpoints are strict bigint milliseconds and end is explicit null when absent.
pub struct DateValue {
    #[serde(deserialize_with = "deserialize_i64_bigint")]
    pub start: i64,
    #[serde(deserialize_with = "deserialize_optional_i64_bigint")]
    pub end: Option<i64>,
    pub include_time: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
/// Number retains NaN, infinities and signed zero. Date uses bigint milliseconds.
/// Ordinary nested null is represented by None, serialized as JavaScript null.
pub enum Value {
    Number(f64), String(String), Boolean(bool),
    /// Legacy Date means end=null and include_time=true; both variants have semantic type Date.
    Date(#[serde(deserialize_with = "deserialize_i64_bigint")] i64),
    DateValue(DateValue),
    List(Vec<Option<Value>>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// validity[i] distinguishes an ordinary null from values[i]; row errors are separate.
pub struct ColumnData<T> { pub values: Vec<T>, pub validity: Vec<bool> }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub enum Column {
    Number(ColumnData<f64>), String(ColumnData<String>), Boolean(ColumnData<bool>),
    Date(#[serde(deserialize_with = "deserialize_date_column")] ColumnData<i64>),
    /// Shares ColumnKind::Date with legacy Date; output storage follows the Engine contract.
    DateValue(ColumnData<DateValue>),
    List(ColumnData<Vec<Option<Value>>>), Union(ColumnData<Value>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ColumnKind { Number, String, Boolean, Date, List, Union }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// One caller-provided time and timezone snapshot per request. now is a strict bigint.
pub struct RuntimeContext {
    #[serde(deserialize_with = "deserialize_i64_bigint")]
    pub now: i64,
    pub time_zone: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// columns must be a JavaScript Map; records and other iterables are rejected.
pub struct EvaluateInput {
    pub row_ids: Vec<RowId>,
    #[serde(deserialize_with = "deserialize_columns")]
    #[ts(type = "Map<PropertyId, Column>")]
    pub columns: HashMap<PropertyId, Column>,
    pub runtime: RuntimeContext,
    pub formula_ids: Vec<PropertyId>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
/// formulas is a JavaScript Map, preserving IDs such as "__proto__".
pub struct EvaluateResult {
    #[ts(type = "Map<PropertyId, { Ok: FormulaOutput } | { Err: FormulaEvaluationError }>")]
    pub formulas: HashMap<PropertyId, Result<FormulaOutput, FormulaEvaluationError>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct FormulaOutput { pub output_type: ValueType, pub column: Column, pub errors: Vec<RowError> }
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct RowError { pub row_index: u32, pub origin_formula_id: PropertyId, pub error: RuntimeError }
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub enum RuntimeError {
    InvalidValueType { expected: ValueType, actual: ValueType },
    InvalidValue { actual: Value, constraint: String },
    InvalidRegex { pattern: String, detail: String },
    InvalidDateText { text: String }, DateOutOfRange,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum FormulaEvaluationError { NotReady }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum FormulaEngineInitError { EmptyId, DuplicateId(PropertyId) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum EngineChangeError { EmptyId }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum CreateDraftError { EmptyId }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum EvaluateInputError {
    InvalidNow { now: i64 }, InvalidTimeZone { time_zone: String },
    EmptyRowId { row_index: u32 }, DuplicateRowId { id: RowId }, EmptyFormulaIds,
    InvalidFormulaId { id: PropertyId }, DuplicateFormulaId { id: PropertyId },
    MissingInputs { ids: Vec<PropertyId> }, UnexpectedInputs { ids: Vec<PropertyId> },
    InvalidColumnType { id: PropertyId, expected: ColumnKind, actual: ColumnKind },
    InvalidColumnLength { id: PropertyId, expected: u32, values_len: u32, validity_len: u32 },
    InvalidValueType { id: PropertyId, row_index: u32, element_path: Vec<u32>, expected: ValueType, actual: ValueType },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CompletionConfig { pub preferred_limit: u32 }
#[derive(Serialize, TS)]
pub struct FormulaDraftState {
    pub version: DraftVersion,
    pub definition: FormulaDefinition,
    pub output_type: ValueType,
    pub diagnostics: Vec<ExpressionDiagnostic>,
    pub tokens: Vec<Token>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ExpressionDiagnostic { pub id: DiagnosticId, pub span: Span, pub message: String }
#[derive(Serialize, TS)]
pub struct CompletionResult { pub items: Vec<CompletionItem>, pub replace: Span, pub preferred_indices: Vec<u32> }
#[derive(Serialize, TS)]
pub struct SignatureHelp { pub signatures: Vec<SignatureItem>, pub active_signature: u32, pub active_parameter: u32 }
#[derive(Serialize, TS)]
pub struct CursorHelp { pub base_version: DraftVersion, pub completion: CompletionResult, pub signature_help: Option<SignatureHelp> }
#[derive(Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// Edits use the original source; base_version must be a bigint.
pub struct FormulaEdit {
    #[serde(deserialize_with = "deserialize_u64_bigint")]
    pub base_version: DraftVersion,
    pub edits: Vec<TextEdit>,
}
#[derive(Serialize, Deserialize, TS)]
pub enum ExpressionUpdate { Replace(String), Edits { edit: FormulaEdit, cursor: TextOffset } }
#[derive(Serialize, TS)]
pub struct QuickFix { pub title: String, pub edit: FormulaEdit }
#[derive(Serialize, TS)]
pub struct UpdateExpressionResult { pub state: FormulaDraftState, pub cursor: TextOffset }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub enum UpdateExpressionError { VersionMismatch, InvalidCursor, InvalidEditRange, OverlappingEdits }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct InvalidDtoPayload { pub operation: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct DraftClosedPayload { pub handle: u32 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ActiveDraftsPayload { pub count: u32 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct EngineInitPayload { pub error: FormulaEngineInitError }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct EngineChangePayload { pub error: EngineChangeError }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct CreateDraftPayload { pub error: CreateDraftError }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct EvaluateInputPayload { pub error: EvaluateInputError }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct UpdateExpressionPayload { pub error: UpdateExpressionError }

/// Controlled WASM failures are cloned across the Worker boundary as plain data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FormulaClientErrorData {
    InvalidDto { message: String, payload: InvalidDtoPayload },
    EngineClosed { message: String, payload: () },
    DraftClosed { message: String, payload: DraftClosedPayload },
    ActiveDrafts { message: String, payload: ActiveDraftsPayload },
    EngineInit { message: String, payload: EngineInitPayload },
    EngineChange { message: String, payload: EngineChangePayload },
    CreateDraft { message: String, payload: CreateDraftPayload },
    EvaluateInput { message: String, payload: EvaluateInputPayload },
    UpdateExpression { message: String, payload: UpdateExpressionPayload },
    FormatError { message: String, payload: () },
    SerializeError { message: String, payload: () },
    /// Emitted by the JS host when the Worker crashes or its transport fails.
    WorkerFailure { message: String, payload: () },
    /// Emitted by the JS host for a malformed Worker response.
    InvalidResponse { message: String, payload: () },
    /// Emitted by the JS host for an invalid RPC request.
    InvalidRequest { message: String, payload: () },
    /// Emitted by the JS host when creating the Worker or loading WASM fails.
    InitializationError { message: String, payload: () },
}
```

### Synchronous WASM session

`FormulaEngineSession` is exported by the initialized WASM package. The Worker owns this allocation and routes Engine and Draft calls to it.

```rust out=analyzer_wasm/src/engine_session.h.rs
use wasm_bindgen::prelude::*;

/// One owned Engine and its Draft handles; all domain work runs in Rust.
/// Explicit close is idempotent. The host must release the generated allocation
/// with free() after closing and must not use the allocation after free().
#[wasm_bindgen]
#[spec::private_fields]
pub struct FormulaEngineSession {}

#[wasm_bindgen]
#[spec::header]
impl FormulaEngineSession {
    /// Constructs an Engine from the lossless FormulaSchema DTO.
    #[wasm_bindgen(constructor)]
    pub fn new(schema: JsValue) -> Result<FormulaEngineSession, JsValue>;
    pub fn get_property(&self, id: String) -> Result<JsValue, JsValue>;
    pub fn get_properties(&self) -> Result<JsValue, JsValue>;
    pub fn get_state(&self) -> Result<JsValue, JsValue>;
    /// Accepts PropertyId[] and returns PropertyId[] with FormulaEngine::required_inputs semantics.
    /// Malformed arrays reject with INVALID_DTO; formula-selection errors use EVALUATE_INPUT.
    /// Available while Drafts are active; Engine close rejects with ENGINE_CLOSED.
    pub fn required_inputs(&self, formula_ids: JsValue) -> Result<JsValue, JsValue>;
    /// Rejects ACTIVE_DRAFTS while any Draft handle exists.
    pub fn upsert(&mut self, property: JsValue) -> Result<JsValue, JsValue>;
    /// Rejects ACTIVE_DRAFTS while any Draft handle exists.
    pub fn remove(&mut self, id: String) -> Result<JsValue, JsValue>;
    pub fn evaluate(&self, input: JsValue) -> Result<JsValue, JsValue>;
    pub fn create_draft(&mut self, formula: JsValue) -> Result<u32, JsValue>;
    pub fn draft_state(&self, handle: u32) -> Result<JsValue, JsValue>;
    /// Cursor is validated as a finite unsigned 32-bit integer before UTF-16 conversion.
    pub fn draft_help(&self, handle: u32, cursor: JsValue, config: JsValue) -> Result<JsValue, JsValue>;
    pub fn draft_quick_fixes(&self, handle: u32, diagnostic_id: String) -> Result<JsValue, JsValue>;
    pub fn draft_format_edits(&self, handle: u32) -> Result<JsValue, JsValue>;
    pub fn draft_update_expression(&mut self, handle: u32, update: JsValue) -> Result<JsValue, JsValue>;
    /// Consumes the Draft; no saved definition is changed.
    pub fn draft_into_definition(&mut self, handle: u32) -> Result<JsValue, JsValue>;
    /// Discards this Draft. Repeated close and close after Engine close succeed.
    pub fn draft_close(&mut self, handle: u32);
    /// Releases all Drafts before releasing Engine; repeated close succeeds.
    pub fn close(&mut self);
}
```

## Current: Synchronous Analyzer

```ts
// The initialized generated package exports Analyzer; the exact signatures of default async initializer / initSync are not a stable contract.
declare class Analyzer {
  constructor(config: AnalyzerConfig);
  analyze(source: string): AnalyzeResult;
  format(source: string, cursor_utf16: number): ApplyResult;
  apply_edits(source: string, edits: TextEdit[], cursor_utf16: number): ApplyResult;
  help(source: string, cursor_utf16: number): HelpResult;
  free(): void;
}
// Retains only fixed properties/preferred_limit; does not store source, results, edit history, or cursor.
// Can be reused for multiple documents with the same configuration; has no configuration update method.
// When the host has Symbol.dispose, the generated glue also exposes the corresponding destructor method.
// Do not call after free/dispose; the resulting failure is outside the controlled contract.

type Ty = "Number" | "String" | "Boolean" | "Date" | { List: Ty };
type Property = { name: string; type: Ty };
type AnalyzerConfig = { properties: Property[]; preferred_limit: number | null };
// These are generated TS declarations; both fields are required. The accepted JS runtime range is described below.
```

```text
config
  Omitted properties → []; explicit undefined → invalid.
  Supports Array<Property>; serde's incidental acceptance of other iterables is not a stable guarantee.
  Omitted/undefined/null preferred_limit → 5; 0 → no preferred_indices.
  A specified value must be a non-negative integer within the WASM usize range; missing property fields, invalid types, or invalid shape → the entire config fails.
  Unknown top-level fields are rejected (including functions); extra fields inside Property are ignored; property names must be unique.
  The builtin set is fixed; configuration cannot extend or replace it. TS callers should satisfy the generated type.
```

### Current DTO

```ts
type Span = { start: number; end: number };
type TextEdit = { range: Span; new_text: string };
type ApplyResult = { source: string; cursor: number };
type DiagnosticKind = "error";
type CodeAction = { title: string; edits: TextEdit[] };
type Diagnostic = {
  kind: DiagnosticKind; message: string; span: Span;
  line: number; col: number; actions: CodeAction[];
};
type Token = {
  kind: string;
  /** Original source spelling, including quotes and escapes. */
  text: string;
  span: Span;
};
type AnalyzeResult = {
  diagnostics: Diagnostic[]; tokens: Token[]; output_type: string;
};
type CompletionItemKind =
  | "FunctionGeneral" | "FunctionText" | "FunctionNumber" | "FunctionDate"
  | "FunctionPeople" | "FunctionList" | "FunctionSpecial" | "Builtin" | "Property" | "Operator";
type CompletionItem = {
  label: string; kind: CompletionItemKind; insert_text: string;
  primary_edit: TextEdit | null; cursor: number | null; additional_edits: TextEdit[];
  detail: string | null; is_disabled: boolean; disabled_reason: string | null;
};
type CompletionResult = {
  items: CompletionItem[]; replace: Span; preferred_indices: number[];
};
type DisplaySegment =
  | { kind: "Name"; text: string } | { kind: "Punct"; text: string }
  | { kind: "Separator"; text: string } | { kind: "Ellipsis" }
  | { kind: "Arrow"; text: string }
  | { kind: "Param"; name: string; ty: string; param_index: number | null }
  | { kind: "ReturnType"; text: string };
type SignatureItem = { segments: DisplaySegment[] };
type SignatureHelp = {
  signatures: SignatureItem[]; active_signature: number; active_parameter: number;
};
type HelpResult = { completion: CompletionResult; signature_help: SignatureHelp | null };
```

```text
analyze
  Formula problems are returned as diagnostics; invalid formulas do not throw.
  Internal diagnostic codes/labels/notes are not exposed; kind currently has only "error".
  Comments/newlines are removed from tokens, but Eof is retained; Token.kind is an open string.
  output_type is always a string; failed/unknown inference is "unknown", not null.
help
  Tolerates incomplete source; arrays such as items/additional_edits/preferred_indices are always present.
  Note the current discrepancy: generated TS maps Option to null, but the actual serializer retains the field and outputs undefined.
  This affects signature_help, primary_edit, cursor, detail, disabled_reason, and param_index.
format / apply_edits
  Return the updated complete source and cursor; do not store source or modify Analyzer configuration.
```

Candidate, ordering, diagnostic-order, formatting, and editing semantics belong to the [Current section of IDE](ide.md).

### Current Coordinates and Exceptions

```text
strings
  source/property name/new_text, etc. must be valid Unicode, with no unpaired surrogate.
  The generated boundary replaces an unpaired surrogate with U+FFFD; inputs relying on this normalization are unsupported.
positions
  All JS cursor/span/edit endpoints are UTF-16 code units, using half-open ranges [start,end).
  Input ranges are based on the original source; returned cursor is based on the returned new source.
  Supported numeric values are finite integers 0..=4_294_967_295.
  Invalid numeric values for a direct cursor parameter may first be coerced by the ABI; such inputs are unsupported.
  Invalid numeric values in an edit DTO → Invalid edits.
  A position inside a surrogate pair → floored to the start of that scalar; this applies to cursor and both endpoints.
  Thus a non-empty UTF-16 range may collapse to empty; it is not always rejected.
past end
  help cursor → clamp to end of document
  format/apply_edits cursor → Invalid cursor
  apply_edits endpoint → Invalid edit range
diagnostic location
  line/col start at 1; col counts Unicode scalars, not UTF-16; an emoji counts as 1 in col and 2 in span.

controlled failures
  constructor → throws primitive string "Invalid analyzer config"
  methods → Error("Invalid edits" | "Invalid cursor" | "Invalid edit range" |
                "Overlapping edits" | "Format error" | "Serialize error")
  Reversed/out-of-bounds range → Invalid edit range; lexer/parser diagnostic → Format error.
  The only controlled exceptions from analyze/help are Serialize error; formula/incomplete-source problems are returned as data.

validation order
  constructor → object/top-level field validation → deserialize → construct
  analyze     → analyze/convert → serialize
  format      → cursor validation/conversion → format → serialize
  apply_edits → deserialize edits → validate/convert ranges in input order → validate/convert cursor
              → sort/check overlap/apply → serialize
  help        → clamp/floor cursor → help → serialize
  Endpoint flooring happens before overlap checks; when multiple errors coexist, the order above determines which one is reported first.
```

The synchronous Analyzer boundary does not expose the evaluator, plan, or business rows, and does not define demo UI policy.
Implementation anchors: [exports](../../analyzer_wasm/src/lib.rs), [DTO](../../analyzer_wasm/src/dto/v1.rs),
[coordinates](../../analyzer_wasm/src/offsets.rs),
[generated TS](../../examples/vite/src/analyzer/generated/wasm_dto.ts).
