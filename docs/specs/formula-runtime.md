---
doc_id: specs.formula-runtime
title: "FormulaEngine: compilation and evaluation"
language: en
source_language: zh-CN
counterpart: ./formula-runtime.zh-CN.md
implementation_status: planned
document_status: draft
translation_status: synced
last_verified: 2026-09-18
---

# FormulaEngine: compilation and evaluation

[简体中文](formula-runtime.zh-CN.md) · [Specification index](README.md)

> Planned: a Rust interface draft, not a shipped API. Error enum variants still need specification.

## Entry point

```rust
// Owns Schema, formulas, dependency analysis, and compiled results; never stores business rows.
// No public graph, revision, or cache; v1 has no batch transactions, streams, paging, cancellation, or row provider.
pub struct FormulaEngine { /* private */ }

impl FormulaEngine {
    // Formula input order does not affect dependency analysis.
    // Duplicate IDs, property/formula ID collisions, unsupported Schema types → Err.
    // Formula syntax/type errors, missing dependencies, cycles → retain definitions; report in state.
    pub fn new(schema: Schema, formulas: Vec<FormulaDefinition>)
        -> Result<Self, FormulaEngineInitError>;
    pub fn state(&self) -> &FormulaEngineState;

    // Reanalyze the formula itself and direct/transitive dependents before returning; identical content is a no-op.
    // ID belongs to the other definition kind → Err, leaving Engine unchanged.
    pub fn upsert_property(&mut self, property: PropertySchema)
        -> Result<ChangeResult, EngineChangeError>;
    pub fn upsert_formula(&mut self, formula: FormulaDefinition)
        -> Result<ChangeResult, EngineChangeError>;
    // Invalid source is still saved; it is not an EngineChangeError.

    // Absent → None; present → remove and mark formulas still depending on it Blocked.
    pub fn remove_property(&mut self, id: &PropertyId) -> Option<ChangeResult>;
    pub fn remove_formula(&mut self, id: &PropertyId) -> Option<ChangeResult>;

    // Uses self's compiled state; only the targets' dependency closure must be Ready.
    // Unrelated Invalid/Blocked formulas do not prevent evaluation.
    // Invalid input or any unrunnable target → Err, with no partial result.
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateError>;

    // Creates a detached editing snapshot without changing Engine; see FormulaDraft.
    pub fn create_draft(&self, formula: FormulaDefinition)
        -> Result<FormulaDraft, CreateDraftError>;
}

// Insert A (depends on absent B) → A: Blocked
// Insert runnable B             → reanalyze A → A: Ready
// Rust ownership/Drop controls lifetime; see WASM API for Worker close().
```

[FormulaDraft](formula-draft.md) owns editing and commit behavior;
[WASM API](formula-runtime-wasm.md) owns Worker and shutdown behavior.

## Definitions and compilation state

```rust
pub struct PropertyId(String); // Shared by properties/formulas; nonempty, case-sensitive, no Unicode normalization
pub struct RowId(String);
pub struct DiagnosticId(String);
pub struct Span { pub start: usize, pub end: usize } // UTF-8 bytes, [start, end)

pub struct Schema {
    pub properties: Vec<PropertySchema>, // Unique IDs
}
pub struct PropertySchema {
    pub id: PropertyId,
    pub ty: Type,
}
pub enum Type { // Concrete static types of non-null values; every field is nullable
    Number, String, Boolean, Date,
    List(Box<Type>),
    Union(Vec<Type>),
    // No public Null/Unknown; Unknown is only an internal inference state.
}
pub struct FormulaDefinition {
    pub id: PropertyId, // No separate name
    pub source: String,
}
pub struct FormulaDiagnostic {
    pub id: DiagnosticId,
    pub code: DiagnosticCode, // Variants pending; do not branch on message text
    pub message: String,
    pub span: Option<Span>,
}

pub struct FormulaEngineState {
    pub schema: Schema,
    pub formulas: Vec<FormulaState>, // Deterministically sorted by definition.id
}
pub struct FormulaState {
    pub definition: FormulaDefinition,
    pub status: FormulaStatus,
    pub output_type: Option<Type>, // Ready requires Some(Type)
    pub diagnostics: Vec<FormulaDiagnostic>,
}
pub enum FormulaStatus {
    Ready,   // Formula and dependencies are runnable
    Invalid, // Own syntax or type errors
    Blocked, // Missing/unrunnable dependencies or a cycle
}
pub struct ChangeResult {
    // Deterministically sorted by ID; empty for a no-op.
    // upsert_formula includes itself; remove_formula includes only surviving dependents.
    pub affected_formulas: Vec<PropertyId>,
}
```

## Columnar evaluation

```rust
pub struct EvaluateInput {
    pub row_ids: Vec<RowId>, // Each ID nonempty and unique within this batch; zero rows allowed
    pub columns: Vec<InputColumn>, // Unique Schema property IDs only; column order is immaterial
    // Supply every column needed by the targets' closure; other Schema columns may be supplied and ignored.
    pub runtime: RuntimeContext, // One snapshot shared by every row and formula in this request
    pub targets: Vec<PropertyId>, // Required, nonempty, unique formula IDs
}
pub struct InputColumn {
    pub id: PropertyId,
    pub data: Column, // Variant must match PropertySchema.ty
}
pub struct RuntimeContext {
    pub evaluated_at_epoch_ms: i64, // UTC Unix epoch milliseconds
    pub timezone_offset_minutes: i32,
}

pub struct ColumnData<T> {
    pub values: Vec<T>, // Length == row_ids.len(); never read placeholders at invalid positions
    pub validity: Validity,
}
pub enum Validity {
    AllValid,
    AllNull, // values still contains a typed placeholder for every row
    Bitmap(Vec<bool>), // Length == values.len(); false means null or row error
}
pub enum Value {
    Number(f64), String(String), Boolean(bool),
    Date(i64), // UTC Unix epoch milliseconds
    List(Vec<Option<Value>>),
}
pub enum Column { // All-null columns retain their variant
    Number(ColumnData<f64>),
    String(ColumnData<String>),
    Boolean(ColumnData<bool>),
    Date(ColumnData<i64>),
    List(ColumnData<Vec<Option<Value>>>),
    Union(ColumnData<Value>),
}

pub struct EvaluateResult {
    pub targets: Vec<TargetResult>, // Full columns for all targets, in input.targets order
}
pub struct TargetResult {
    pub id: PropertyId,
    pub output_type: Type, // Concrete non-null static type
    pub column: Column, // Both ordinary null and row errors mark the position invalid
    pub errors: Vec<RowError>, // Ordinary null has no error; one row may have multiple errors
    // Deterministic evaluation-traversal order, without an additional row sort.
}
pub struct RowError {
    pub row_index: usize, // Position in input.row_ids
    pub origin_formula_id: PropertyId, // Actual failing formula, possibly a target dependency
    pub code: RowErrorCode, // Variants pending
    pub message: String,
}
```

See [formula grammar](formula-language.md) for expressions, references, and row semantics,
and [builtins](builtin-functions.md) for signatures. Their Current behavior does not imply this API is implemented.
