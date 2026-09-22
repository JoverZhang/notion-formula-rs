---
doc_id: specs.formula-engine
title: "FormulaEngine: compilation and evaluation"
language: en
source_language: zh-CN
counterpart: ./formula-engine.zh-CN.md
implementation_status: planned
document_status: draft
translation_status: needs-update
last_verified: 2026-09-19
---

# FormulaEngine: compilation and evaluation

- [简体中文](formula-engine.zh-CN.md)
- [Specification index](README.md)

> Planned: not yet released; error enum variants are not finalized.

## FormulaEngine

```rust
/// # Examples
///
/// ```
/// use std::time::{SystemTime, UNIX_EPOCH};
///
/// // 1. Define schema
/// let schema = Schema {
///     properties: vec![
///         PropertyDefinition::Input { id: "text".into(), ty: Type::String },
///         PropertyDefinition::Input { id: "number".into(), ty: Type::Number },
///         PropertyDefinition::Formula(FormulaDefinition {
///             id: "formula".into(),
///             expression: r#"repeat(prop("text"), prop("number"))"#.into(),
///         }),
///     ],
/// };
///
/// // 2. Create the engine
/// let engine = FormulaEngine::new(schema).expect("valid definitions");
///
/// // 3. Capture one runtime snapshot (this example uses UTC+08:00)
/// let since_epoch = SystemTime::now()
///     .duration_since(UNIX_EPOCH)
///     .expect("system clock is before Unix epoch");
/// let runtime = RuntimeContext {
///     evaluated_at_epoch_ms: since_epoch.as_millis().try_into().expect("timestamp exceeds i64"),
///     timezone_offset_minutes: 8 * 60, // Use the business/user time zone's offset.
/// };
///
/// // 4. Build input columns in row_ids order
/// // row_id    text    number
/// // row-1     "ha"    2
/// // row-2     "go"    3
/// let input = EvaluateInput {
///     row_ids: vec!["row-1".into(), "row-2".into()],
///     columns: vec![
///         InputColumn {
///             id: "text".into(),
///             data: Column::String(ColumnData {
///                 values: vec!["ha".into(), "go".into()],
///                 validity: Validity::AllValid,
///             }),
///         },
///         InputColumn {
///             id: "number".into(),
///             data: Column::Number(ColumnData {
///                 values: vec![2.0, 3.0],
///                 validity: Validity::AllValid,
///             }),
///         },
///     ],
///     runtime,
///     targets: vec!["formula".into()],
/// };
///
/// // 5. Evaluate the target formula
/// let result = engine.evaluate(&input).expect("valid request");
///
/// // 6. Read the result column in row_ids order
/// let Ok(output) = &result.targets[0].result else {
///     panic!("expected a computed target");
/// };
/// let Column::String(column) = &output.column else {
///     panic!("expected a string column");
/// };
/// assert_eq!(column.values, ["haha", "gogogo"]);
/// assert!(output.errors.is_empty());
/// ```
pub struct FormulaEngine {}

impl FormulaEngine {
    /// - allow: Definitions in any order.
    /// - allow: Syntax/type errors, missing or unrunnable dependencies, and cycles → Ok, with definitions retained. - See state()
    /// - error: Empty or duplicate IDs.
    pub fn new(schema: Schema) -> Result<Self, FormulaEngineInitError>;

    /// Inspect the current FormulaEngine state.
    ///
    /// Inspect status and diagnostics through state().formulas.
    pub fn state(&self) -> &FormulaEngineState;

    /// Atomically updates a PropertyDefinition.
    /// Reanalyzes the ID's direct and transitive formula dependents and any new Formula definition before returning.
    ///
    /// - no-op: The definition content has not changed.
    /// - allow: Switching between Input and Formula under the same ID.
    /// - allow: Formula definitions with syntax/type errors, missing or unrunnable dependencies, or cycles. - See state()
    /// - error: The ID is an empty string.
    pub fn upsert(&mut self, property: PropertyDefinition)
        -> Result<ChangeResult, EngineChangeError>;

    /// Formulas still depending on the removed ID become Blocked.
    pub fn remove(&mut self, id: &PropertyId) -> Option<ChangeResult>;

    /// Returns a separate result for each target; one target's failure does not affect the others.
    ///
    /// - allow: Invalid or Blocked formulas in targets.
    /// - allow: Missing Input columns; only targets depending on them fail.
    /// - error: Input violates EvaluateInput's ID, type, or length constraints; no target is evaluated.
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateError>;

    /// Analyze a candidate against Engine; edits do not change Engine.
    ///
    /// - allow: New IDs; replacing a same-ID Input or Formula for analysis within Draft.
    /// - allow: Formula definitions with syntax/type errors, missing or unrunnable dependencies, or cycles. - See FormulaDraft::state()
    /// - error: The ID is an empty string.
    ///
    /// # Examples
    ///
    /// ```
    /// // 1. Create an engine with a saved formula
    /// let mut engine = FormulaEngine::new(Schema {
    ///     properties: vec![PropertyDefinition::Formula(FormulaDefinition {
    ///         id: "formula".into(),
    ///         expression: "1 + 1".into(),
    ///     })],
    /// }).expect("valid definitions");
    ///
    /// // 2. Create a draft from the saved definition
    /// let definition = engine.state().formulas[0].definition.clone();
    /// let mut draft = engine.create_draft(definition).expect("valid draft ID");
    /// assert_eq!(draft.state().definition.expression, "1 + 1");
    ///
    /// // 3. Edit the draft; the engine keeps the saved definition
    /// draft.set_expression("1 + 2".into());
    /// assert_eq!(draft.state().definition.expression, "1 + 2");
    /// assert!(draft.state().diagnostics.is_empty());
    /// assert_eq!(engine.state().formulas[0].definition.expression, "1 + 1");
    ///
    /// // 4. Finish editing and save the definition
    /// let definition = draft.into_definition();
    /// engine.upsert(PropertyDefinition::Formula(definition)).expect("valid definition");
    /// assert_eq!(engine.state().formulas[0].definition.expression, "1 + 2");
    /// ```
    pub fn create_draft(&self, formula: FormulaDefinition)
        -> Result<FormulaDraft<'_>, CreateDraftError>;
}
```

- [FormulaDraft](ide.md)
- [WASM API](wasm-api.md)

## Schema and PropertyDefinition

```rust
/// A stable, caller-assigned ID with no required string format; Input and Formula share one namespace.
/// From conversions preserve the original string; the Engine checks that IDs are nonempty and unique within Schema.
/// Case-sensitive and not Unicode-normalized. The host owns display names and preserves IDs when renaming.
#[derive(Clone)]
pub struct PropertyId(String);

impl From<&str> for PropertyId {
    fn from(value: &str) -> Self;
}
impl From<String> for PropertyId {
    fn from(value: String) -> Self;
}

/// Caller-assigned; From conversions preserve the original string, and row IDs are validated during evaluation.
pub struct RowId(String);

impl From<&str> for RowId {
    fn from(value: &str) -> Self;
}
impl From<String> for RowId {
    fn from(value: String) -> Self;
}

pub struct DiagnosticId(String);

/// UTF-8 byte offsets defining the interval [start, end).
pub struct Span { pub start: usize, pub end: usize }

/// Every property allows null.
pub struct Schema {
    pub properties: Vec<PropertyDefinition>,
}
pub enum PropertyDefinition {
    /// Values come from the input columns of each evaluate call.
    Input { id: PropertyId, ty: Type },
    /// Values are evaluated from expression; the Engine infers the type.
    Formula(FormulaDefinition),
}
/// The static type of a non-null value.
pub enum Type {
    Number, String, Boolean, Date,
    List(Box<Type>),
    Union(Vec<Type>),
}
#[derive(Clone)]
pub struct FormulaDefinition {
    pub id: PropertyId,
    /// prop("id") refers by ID to an Input or Formula in the same Schema.
    pub expression: String,
}
```

## Compilation state

```rust
pub struct FormulaDiagnostic {
    pub id: DiagnosticId,
    pub code: DiagnosticCode,
    /// For display only; use code to identify the error programmatically.
    pub message: String,
    pub span: Option<Span>,
}

pub struct FormulaEngineState {
    pub schema: Schema,
    /// Corresponds to the Formula definitions in Schema, sorted by definition.id.
    pub formulas: Vec<FormulaState>,
}
pub struct FormulaState {
    pub definition: FormulaDefinition,
    pub status: FormulaStatus,
    /// Must have a value when Ready.
    pub output_type: Option<Type>,
    pub diagnostics: Vec<FormulaDiagnostic>,
}
pub enum FormulaStatus {
    /// The formula and its dependencies can all execute.
    Ready,
    /// The formula itself has a syntax or type error.
    Invalid,
    /// A dependency is missing or cannot execute, or there is a dependency cycle.
    Blocked,
}
pub struct ChangeResult {
    /// Sorted by ID; empty when nothing changed.
    /// After a change, includes the updated ID if its definition is a Formula.
    /// Removal or replacement with Input includes only formula dependents that remain after the change.
    pub affected_formulas: Vec<PropertyId>,
}
```

## Evaluation

### Input

```rust
pub struct EvaluateInput {
    /// IDs are nonempty and unique within the batch; zero rows are allowed.
    pub row_ids: Vec<RowId>,
    /// IDs are unique and refer to Inputs in Schema; column order does not affect evaluation.
    /// Missing required columns produce MissingInputs for the corresponding target; extra Input columns do not participate in evaluation.
    pub columns: Vec<InputColumn>,
    /// Every row and formula in this request shares this snapshot.
    pub runtime: RuntimeContext,
    /// Nonempty, with unique IDs referring to formulas.
    pub targets: Vec<PropertyId>,
}
pub struct InputColumn {
    pub id: PropertyId,
    /// The variant must match the corresponding Input definition's ty.
    pub data: Column,
}
pub struct RuntimeContext {
    /// The UTC Unix timestamp in milliseconds used by now(); the Engine does not read the clock itself.
    /// For real-time evaluation, the caller captures the current time once at request start; tests or replay may supply a fixed time.
    pub evaluated_at_epoch_ms: i64,
    /// Local time minus UTC, in minutes: UTC+08:00 = 480, UTC-05:00 = -300, UTC = 0.
    /// The caller supplies the business/user time zone's offset at evaluated_at_epoch_ms, used by today() and date operations.
    /// The offset stays fixed throughout evaluation; daylight-saving rules are not applied per evaluated date.
    pub timezone_offset_minutes: i32,
}
```

### Column and null

```rust
pub struct ColumnData<T> {
    /// Length equals the number of evaluation rows, including when AllNull.
    /// Invalid positions contain only placeholders, which must not be read.
    pub values: Vec<T>,
    pub validity: Validity,
}
pub enum Validity {
    AllValid,
    AllNull,
    /// Length equals values.len(); false means null or a row error.
    Bitmap(Vec<bool>),
}
pub enum Value {
    Number(f64), String(String), Boolean(bool),
    /// UTC Unix timestamp in milliseconds.
    Date(i64),
    List(Vec<Option<Value>>),
}
/// Retains the column type when every row is null.
pub enum Column {
    Number(ColumnData<f64>),
    String(ColumnData<String>),
    Boolean(ColumnData<bool>),
    Date(ColumnData<i64>),
    List(ColumnData<Vec<Option<Value>>>),
    Union(ColumnData<Value>),
}
```

### Results and row errors

```rust
pub struct EvaluateResult {
    /// One entry per input.targets item, in the same order; errors are returned even when all targets fail.
    pub targets: Vec<TargetResult>,
}
pub struct TargetResult {
    pub id: PropertyId,
    pub result: Result<TargetOutput, TargetError>,
}
pub struct TargetOutput {
    pub output_type: Type,
    /// A complete column; both ordinary nulls and row errors mark their positions as invalid.
    pub column: Column,
    /// Row errors only; one row can have multiple errors.
    /// In deterministic evaluation-traversal order, without an additional sort by row.
    pub errors: Vec<RowError>,
}
pub enum TargetError {
    /// The diagnostics match this target's FormulaState.
    Invalid { diagnostics: Vec<FormulaDiagnostic> },
    /// The diagnostics match this target's FormulaState.
    Blocked { diagnostics: Vec<FormulaDiagnostic> },
    /// The target is Ready, but Input columns required directly or transitively are missing.
    MissingInputs {
        /// All missing Input IDs; nonempty, deduplicated, and sorted by ID.
        property_ids: Vec<PropertyId>,
    },
}
pub struct RowError {
    /// Position in input.row_ids.
    pub row_index: usize,
    /// The formula where the error occurred, which may be a dependency of the target.
    pub origin_formula_id: PropertyId,
    pub code: RowErrorCode,
    pub message: String,
}
```

- [Formula grammar](formula-language.md)
- [Builtins](builtin-functions.md)
