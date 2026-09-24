---
doc_id: specs.formula-engine
title: "FormulaEngine: compilation and evaluation"
language: en
source_language: zh-CN
counterpart: ./formula-engine.zh-CN.md
implementation_status: current
document_status: draft
translation_status: synced
translation_model: gpt-6-sol
translation_review_model: gpt-6-astra
last_verified: 2026-09-24
---

# FormulaEngine: compilation and evaluation

[简体中文](formula-engine.zh-CN.md)

> Current: definition management, dependency analysis, and state queries. Planned: evaluation and FormulaDraft.

**Contents**

- [Type definitions](#type-definitions)
- [FormulaEngine API](#formulaengine-api)

## Type definitions

```rust
use std::collections::HashMap;
```

**Current schema types**

```rust out=formula_engine/src/formula_engine.h.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormulaSchema {
    pub properties: Vec<PropertyDefinition>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyDefinition {
    Input { id: PropertyId, ty: ValueType },
    Formula(FormulaDefinition),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormulaDefinition {
    pub id: PropertyId,
    /// Formula expression; prop("id") references another Property by ID in the same FormulaSchema.
    pub expression: String,
}

/// Stable ID assigned by the caller.
///
/// - Input and Formula share a namespace; Engine validates that IDs are nonempty and unique within FormulaSchema.
/// - No string-format restrictions; case-sensitive, without Unicode normalization.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
#[from(String, &str)]
pub struct PropertyId(pub String);

/// Declared types for Inputs and inferred types for Formulas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    /// See [Planned Number](formula-language.md#planned-number) for numeric behavior.
    Number,
    String, Boolean, Date,
    /// Static type is undetermined; allowed in Input declarations and inferred types, including nested types.
    /// Accepts any Value variant at this position; operations check the concrete type at runtime.
    Unknown,
    List(Box<ValueType>),
    Union(Vec<ValueType>),
}

```

**Planned runtime value types**

```rust
/// Stores runtime data in columns, with a bitmap marking invalid positions.
pub enum Column {
    Number(ColumnData<f64>),
    String(ColumnData<String>),
    Boolean(ColumnData<bool>),
    Date(ColumnData<i64>),
    List(ColumnData<Vec<Option<Value>>>),
    /// Can carry Union or Unknown; each non-null value retains its concrete type.
    Union(ColumnData<Value>),
}
pub struct ColumnData<T> {
    /// Length equals the evaluation row count; null positions hold placeholders that must not be read.
    pub values: Vec<T>,
    /// See [Arrow NullBuffer](https://arrow.apache.org/rust/arrow_buffer/buffer/struct.NullBuffer.html).
    /// Length equals values.len(); both ordinary nulls and row errors are marked null.
    pub validity: NullBuffer,
}
pub enum Value {
    Number(f64), String(String), Boolean(bool),
    /// UTC Unix timestamp in milliseconds.
    Date(i64),
    List(Vec<Option<Value>>),
}

```

**Current state and change types**

```rust out=formula_engine/src/formula_engine.h.rs
#[derive(Debug, PartialEq, Eq)]
pub enum FormulaEngineState<'a> {
    /// No cycles, and every formula is Ready.
    AllReady,
    /// Other formulas may remain Ready when one formula is not.
    NotAllReady {
        /// Returns one cycle when present; each ID directly depends on the next, and the first and last IDs are equal.
        /// - empty: No cycle, but some formulas are not Ready.
        /// - [A, A]: Self-reference A -> A
        /// - [A, B, C, A]: Cycle A -> B -> C -> A
        cycle_path: &'a [PropertyId],
    },
}

/// Independent snapshot of a property definition and its state; later Engine updates do not change it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyState {
    Input { id: PropertyId, ty: ValueType },
    Formula(FormulaState),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormulaState {
    pub definition: FormulaDefinition,
    pub status: FormulaStatus,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormulaStatus {
    /// The formula and its dependencies are executable.
    /// output_type may contain Unknown; for example, [] is Ready with type List(Unknown).
    Ready { output_type: ValueType },
    NotReady,
}

/// Result of a FormulaEngine::upsert / remove change.
#[derive(Debug, PartialEq, Eq)]
pub struct FormulaEngineChangeResult {
    /// Formula IDs affected by this change, including direct and transitive dependents.
    /// - Includes only Formulas that still exist after the change.
    /// - When upsert adds or changes a Formula, includes that Formula itself.
    /// - Empty when the definition has not changed.
    pub affected_formulas: Vec<PropertyId>,
}

/// Invalid definitions supplied to FormulaEngine::new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaEngineInitError {
    EmptyId,
    DuplicateId(PropertyId),
}

/// Invalid definition supplied to FormulaEngine::upsert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineChangeError {
    EmptyId,
}
```

**Planned evaluation request and result types**

```rust
/// Arguments to FormulaEngine::evaluate().
pub struct EvaluateInput {
    /// Values in input and result columns follow this order.
    /// IDs are nonempty and unique within the batch; zero rows are allowed.
    pub row_ids: Vec<RowId>,
    /// Matches every Input in Engine by ID, including columns unused in this evaluation.
    /// evaluate() validates column and nested-value types, and the lengths of values and validity, at runtime.
    pub columns: HashMap<PropertyId, Column>,
    pub runtime: RuntimeContext,
    /// Formula IDs whose results are requested; Engine evaluates their formula dependencies automatically.
    /// Nonempty, without duplicates; every ID must identify a Formula in Engine.
    pub formula_ids: Vec<PropertyId>,
}
#[derive(derive_more::From)]
#[from(String, &str)]
pub struct RowId(pub String);

/// All rows and formulas in one evaluation share this time and time-zone snapshot.
pub struct RuntimeContext {
    /// UTC Unix timestamp in milliseconds used by now(), supplied by the caller; Engine does not read the clock.
    /// For live evaluation, capture the time at request start; tests and replays may use a fixed value.
    pub evaluated_at_epoch_ms: i64,
    /// Local time minus UTC, in minutes: UTC+08:00 = 480, UTC-05:00 = -300.
    /// Use the business/user time zone's offset at evaluated_at_epoch_ms for today() and date operations.
    /// The offset stays fixed throughout evaluation; daylight-saving rules are not applied to the dates being computed.
    pub timezone_offset_minutes: i32,
}

pub struct EvaluateResult {
    /// One entry per input.formula_ids ID, including formulas whose evaluation failed.
    pub formulas: HashMap<PropertyId, Result<FormulaOutput, FormulaEvaluationError>>,
}
/// Even if every row fails, returns Ok(FormulaOutput), with failures recorded in errors.
pub struct FormulaOutput {
    /// Output type at the time of this evaluation.
    pub output_type: ValueType,
    pub column: Column,
    /// A row may have multiple errors.
    /// Error order is consistent for the same definitions and input; sorting by row_index is not guaranteed.
    pub errors: Vec<RowError>,
}
/// A row evaluation failure marks the corresponding result position null; other rows continue.
/// Dependency errors propagate only through executed branches and do not change FormulaStatus.
pub struct RowError {
    /// Index into input.row_ids.
    pub row_index: usize,
    /// Formula ID where the error originated; may identify a dependency of the requested formula.
    pub origin_formula_id: PropertyId,
    pub error: RuntimeError,
}
/// constraint and detail are for display only.
pub enum RuntimeError {
    /// The runtime value's type is not accepted by the current operation.
    InvalidValueType {
        expected: ValueType,
        actual: ValueType,
    },
    /// The type is accepted, but the value violates a function constraint; regex and date failures use the specific variants below.
    InvalidValue {
        actual: Value,
        /// For example, "repeat count must be nonnegative".
        constraint: String,
    },
    InvalidRegex {
        pattern: String,
        /// Explanation of the regex compilation failure.
        detail: String,
    },
    /// Date text cannot be parsed.
    InvalidDateText {
        text: String,
    },
    /// A date operation exceeds the supported range.
    DateOutOfRange,
}
/// Returned when a formula cannot begin evaluation; see RowError for row evaluation failures.
pub enum FormulaEvaluationError {
    /// The Formula's FormulaStatus is NotReady.
    NotReady,
}
```

## FormulaEngine API

### Planned evaluation example

```rust
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use std::time::{SystemTime, UNIX_EPOCH};
///
/// // 1. Define schema
/// let schema = FormulaSchema {
///     properties: vec![
///         PropertyDefinition::Input { id: "text".into(), ty: ValueType::String },
///         PropertyDefinition::Input { id: "number".into(), ty: ValueType::Number },
///         PropertyDefinition::Formula(FormulaDefinition {
///             id: "formula".into(),
///             expression: r#"repeat(prop("text"), prop("number"))"#.into(),
///         }),
///     ],
/// };
///
/// // 2. Create the engine
/// let engine = FormulaEngine::new(schema).expect("valid definitions");
/// assert!(matches!(engine.state(), FormulaEngineState::AllReady));
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
///     columns: HashMap::from([
///         (
///             "text".into(),
///             Column::String(ColumnData {
///                 values: vec!["ha".into(), "go".into()],
///                 validity: NullBuffer::new_valid(2),
///             }),
///         ),
///         (
///             "number".into(),
///             Column::Number(ColumnData {
///                 values: vec![2.0, 3.0],
///                 validity: NullBuffer::new_valid(2),
///             }),
///         ),
///     ]),
///     runtime,
///     formula_ids: vec!["formula".into()],
/// };
///
/// // 5. Evaluate the requested formula
/// let result = engine.evaluate(&input).expect("valid request");
///
/// // 6. Read the result column in row_ids order
/// let Some(Ok(output)) = result.formulas.get(&PropertyId::from("formula")) else {
///     panic!("expected a computed formula");
/// };
/// let Column::String(column) = &output.column else {
///     panic!("expected a string column");
/// };
/// assert_eq!(column.values, ["haha", "gogogo"]);
/// assert!(output.errors.is_empty());
/// ```
```

### Current definition methods

```rust out=formula_engine/src/formula_engine.h.rs
#[spec::private_fields]
pub struct FormulaEngine {}

#[spec::header]
impl FormulaEngine {
    /// allows:
    /// - Definitions in any order.
    /// - Saving formula definitions with syntax/type errors, missing or unready dependencies, or dependency cycles.
    ///
    /// errors:
    /// - Empty ID.
    /// - Duplicate ID.
    pub fn new(schema: FormulaSchema) -> Result<Self, FormulaEngineInitError>;

    pub fn property(&self, id: &PropertyId) -> Option<PropertyState>;
    pub fn properties(&self) -> Vec<PropertyState>;
    pub fn state(&self) -> FormulaEngineState<'_>;

    /// Atomically updates PropertyDefinition.
    /// Reanalyzes direct and transitive formula dependents of this ID, and the new Formula definition, before returning.
    ///
    /// no-op: The definition has not changed.
    ///
    /// allows:
    /// - Switching the same ID between Input and Formula.
    /// - Saving formula definitions with syntax/type errors, missing or unready dependencies, or dependency cycles.
    ///
    /// error: Empty ID string.
    pub fn upsert(&mut self, property: PropertyDefinition)
        -> Result<FormulaEngineChangeResult, EngineChangeError>;

    /// After removal, formulas that still depend on this ID become NotReady.
    pub fn remove(&mut self, id: &PropertyId) -> Option<FormulaEngineChangeResult>;
}
```

### Planned evaluation and Draft methods

```rust
impl FormulaEngine {

    /// Returns Ok(EvaluateResult) once input validation passes, even if every requested formula fails.
    /// Formula and row errors are returned with the result; other formulas continue.
    ///
    /// allows: NotReady formulas in formula_ids.
    ///
    /// errors: Input validation fails; no formulas are evaluated.
    /// - EvaluateInputError::MissingInputs: Missing Input columns; reports all missing IDs, deduplicated and sorted by ID.
    /// - Extra columns, or invalid IDs, types, or lengths.
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateInputError>;

    /// Analyzes the candidate formula against Engine; edits do not modify Engine.
    ///
    /// allows:
    /// - A new ID; replacing the same-ID Input or Formula in the Draft's analysis.
    /// - Formula definitions with syntax/type errors, missing or nonexecutable dependencies, or dependency cycles. See FormulaDraft::state().
    ///
    /// error: Empty ID string.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    /// // 1. Create an engine with a saved formula
    /// let mut engine = FormulaEngine::new(FormulaSchema {
    ///     properties: vec![PropertyDefinition::Formula(FormulaDefinition {
    ///         id: "formula".into(),
    ///         expression: "1 + 1".into(),
    ///     })],
    /// })?;
    ///
    /// // 2. Create a draft from the saved definition
    /// let Some(PropertyState::Formula(saved)) = engine.property(&"formula".into()) else {
    ///     panic!("expected a saved formula");
    /// };
    /// let mut draft = engine.create_draft(saved.definition)?;
    /// assert_eq!(draft.state().definition.expression, "1 + 1");
    ///
    /// // 3. Edit the draft; the engine keeps the saved definition
    /// draft.update_expression(ExpressionUpdate::Replace("1 + 2".into()))?;
    /// assert_eq!(draft.state().definition.expression, "1 + 2");
    /// assert!(draft.state().diagnostics.is_empty());
    /// assert!(matches!(
    ///     engine.property(&"formula".into()),
    ///     Some(PropertyState::Formula(saved)) if saved.definition.expression == "1 + 1"
    /// ));
    ///
    /// // 4. Finish editing and save the definition
    /// let definition = draft.into_definition();
    /// engine.upsert(PropertyDefinition::Formula(definition))?;
    /// let Some(PropertyState::Formula(saved)) = engine.property(&"formula".into()) else {
    ///     panic!("expected a saved formula");
    /// };
    /// assert_eq!(saved.definition.expression, "1 + 2");
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_draft(&self, formula: FormulaDefinition)
        -> Result<FormulaDraft<'_>, CreateDraftError>;
}
```

- [FormulaDraft](ide.md)
