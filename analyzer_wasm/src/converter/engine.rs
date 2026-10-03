//! Lossless conversions for the owned Engine session. Domain validation remains
//! in formula_engine; these conversions only change transport representations.
//! WASM usize offsets, lengths, and indices fit u32; transport uses u32 explicitly
//! because serde serializes usize as u64 regardless of the target pointer width.

use formula_engine as rust;

use super::Converter;
use super::shared::{span_dto, token_view};
use crate::dto::engine as js;
use crate::dto::v1::TextEdit;
use crate::offsets::{utf16_to_8_cursor, utf16_to_8_text_edits};

impl From<js::ValueType> for rust::ValueType {
    fn from(value: js::ValueType) -> Self {
        match value {
            js::ValueType::Number => Self::Number,
            js::ValueType::String => Self::String,
            js::ValueType::Boolean => Self::Boolean,
            js::ValueType::Date => Self::Date,
            js::ValueType::Unknown => Self::Unknown,
            js::ValueType::List(inner) => Self::List(Box::new((*inner).into())),
            js::ValueType::Union(types) => Self::Union(types.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<rust::ValueType> for js::ValueType {
    fn from(value: rust::ValueType) -> Self {
        match value {
            rust::ValueType::Number => Self::Number,
            rust::ValueType::String => Self::String,
            rust::ValueType::Boolean => Self::Boolean,
            rust::ValueType::Date => Self::Date,
            rust::ValueType::Unknown => Self::Unknown,
            rust::ValueType::List(inner) => Self::List(Box::new((*inner).into())),
            rust::ValueType::Union(types) => {
                Self::Union(types.into_iter().map(Into::into).collect())
            }
        }
    }
}

impl From<js::FormulaDefinition> for rust::FormulaDefinition {
    fn from(value: js::FormulaDefinition) -> Self {
        Self {
            id: value.id.into(),
            expression: value.expression,
        }
    }
}

impl From<rust::FormulaDefinition> for js::FormulaDefinition {
    fn from(value: rust::FormulaDefinition) -> Self {
        Self {
            id: value.id.0,
            expression: value.expression,
        }
    }
}

impl From<js::PropertyDefinition> for rust::PropertyDefinition {
    fn from(value: js::PropertyDefinition) -> Self {
        match value {
            js::PropertyDefinition::Input { id, ty } => Self::Input {
                id: id.into(),
                ty: ty.into(),
            },
            js::PropertyDefinition::Formula(definition) => Self::Formula(definition.into()),
        }
    }
}

impl From<js::FormulaSchema> for rust::FormulaSchema {
    fn from(value: js::FormulaSchema) -> Self {
        Self {
            properties: value.properties.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<rust::PropertyState> for js::PropertyState {
    fn from(value: rust::PropertyState) -> Self {
        match value {
            rust::PropertyState::Input { id, ty } => Self::Input {
                id: id.0,
                ty: ty.into(),
            },
            rust::PropertyState::Formula(formula) => Self::Formula(js::FormulaState {
                definition: formula.definition.into(),
                status: match formula.status {
                    rust::FormulaStatus::Ready { output_type } => js::FormulaStatus::Ready {
                        output_type: output_type.into(),
                    },
                    rust::FormulaStatus::NotReady => js::FormulaStatus::NotReady,
                },
            }),
        }
    }
}

impl From<rust::FormulaEngineState<'_>> for js::FormulaEngineState {
    fn from(value: rust::FormulaEngineState<'_>) -> Self {
        match value {
            rust::FormulaEngineState::AllReady => Self::AllReady,
            rust::FormulaEngineState::NotAllReady { cycle_path } => Self::NotAllReady {
                cycle_path: cycle_path.iter().map(|id| id.0.clone()).collect(),
            },
        }
    }
}

impl From<rust::FormulaEngineChangeResult> for js::FormulaEngineChangeResult {
    fn from(value: rust::FormulaEngineChangeResult) -> Self {
        Self {
            affected_formulas: value.affected_formulas.into_iter().map(|id| id.0).collect(),
        }
    }
}

impl From<js::Value> for rust::Value {
    fn from(value: js::Value) -> Self {
        match value {
            js::Value::Number(value) => Self::Number(value),
            js::Value::String(value) => Self::String(value),
            js::Value::Boolean(value) => Self::Boolean(value),
            js::Value::Date(value) => Self::Date(value),
            js::Value::List(values) => {
                Self::List(values.into_iter().map(|v| v.map(Into::into)).collect())
            }
        }
    }
}

impl From<rust::Value> for js::Value {
    fn from(value: rust::Value) -> Self {
        match value {
            rust::Value::Number(value) => Self::Number(value),
            rust::Value::String(value) => Self::String(value),
            rust::Value::Boolean(value) => Self::Boolean(value),
            rust::Value::Date(value) => Self::Date(value),
            rust::Value::List(values) => {
                Self::List(values.into_iter().map(|v| v.map(Into::into)).collect())
            }
        }
    }
}

fn native_column<T, U>(column: js::ColumnData<T>, convert: impl Fn(T) -> U) -> rust::ColumnData<U> {
    rust::ColumnData {
        values: column.values.into_iter().map(convert).collect(),
        validity: rust::NullBuffer::from(column.validity),
    }
}

fn js_column<T, U>(column: rust::ColumnData<T>, convert: impl Fn(T) -> U) -> js::ColumnData<U> {
    js::ColumnData {
        values: column.values.into_iter().map(convert).collect(),
        validity: column.validity.iter().collect(),
    }
}

impl From<js::Column> for rust::Column {
    fn from(value: js::Column) -> Self {
        match value {
            js::Column::Number(column) => Self::Number(native_column(column, |v| v)),
            js::Column::String(column) => Self::String(native_column(column, |v| v)),
            js::Column::Boolean(column) => Self::Boolean(native_column(column, |v| v)),
            js::Column::Date(column) => Self::Date(native_column(column, |v| v)),
            js::Column::List(column) => Self::List(native_column(column, |v| {
                v.into_iter().map(|v| v.map(Into::into)).collect()
            })),
            js::Column::Union(column) => Self::Union(native_column(column, Into::into)),
        }
    }
}

impl From<rust::Column> for js::Column {
    fn from(value: rust::Column) -> Self {
        match value {
            rust::Column::Number(column) => Self::Number(js_column(column, |v| v)),
            rust::Column::String(column) => Self::String(js_column(column, |v| v)),
            rust::Column::Boolean(column) => Self::Boolean(js_column(column, |v| v)),
            rust::Column::Date(column) => Self::Date(js_column(column, |v| v)),
            rust::Column::List(column) => Self::List(js_column(column, |v| {
                v.into_iter().map(|v| v.map(Into::into)).collect()
            })),
            rust::Column::Union(column) => Self::Union(js_column(column, Into::into)),
        }
    }
}

impl From<js::EvaluateInput> for rust::EvaluateInput {
    fn from(value: js::EvaluateInput) -> Self {
        Self {
            row_ids: value.row_ids.into_iter().map(Into::into).collect(),
            columns: value
                .columns
                .into_iter()
                .map(|(id, column)| (id.into(), column.into()))
                .collect(),
            runtime: rust::RuntimeContext {
                now: value.runtime.now,
                time_zone: value.runtime.time_zone,
            },
            formula_ids: value.formula_ids.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<rust::EvaluateResult> for js::EvaluateResult {
    fn from(value: rust::EvaluateResult) -> Self {
        Self {
            formulas: value
                .formulas
                .into_iter()
                .map(|(id, result)| (id.0, result.map(Into::into).map_err(Into::into)))
                .collect(),
        }
    }
}

impl From<rust::FormulaOutput> for js::FormulaOutput {
    fn from(value: rust::FormulaOutput) -> Self {
        Self {
            output_type: value.output_type.into(),
            column: value.column.into(),
            errors: value
                .errors
                .into_iter()
                .map(|e| js::RowError {
                    row_index: e.row_index as u32,
                    origin_formula_id: e.origin_formula_id.0,
                    error: e.error.into(),
                })
                .collect(),
        }
    }
}

impl From<rust::RuntimeError> for js::RuntimeError {
    fn from(value: rust::RuntimeError) -> Self {
        match value {
            rust::RuntimeError::InvalidValueType { expected, actual } => Self::InvalidValueType {
                expected: expected.into(),
                actual: actual.into(),
            },
            rust::RuntimeError::InvalidValue { actual, constraint } => Self::InvalidValue {
                actual: actual.into(),
                constraint,
            },
            rust::RuntimeError::InvalidRegex { pattern, detail } => {
                Self::InvalidRegex { pattern, detail }
            }
            rust::RuntimeError::InvalidDateText { text } => Self::InvalidDateText { text },
            rust::RuntimeError::DateOutOfRange => Self::DateOutOfRange,
        }
    }
}

impl From<rust::FormulaEvaluationError> for js::FormulaEvaluationError {
    fn from(value: rust::FormulaEvaluationError) -> Self {
        match value {
            rust::FormulaEvaluationError::NotReady => Self::NotReady,
        }
    }
}

impl From<rust::FormulaEngineInitError> for js::FormulaEngineInitError {
    fn from(value: rust::FormulaEngineInitError) -> Self {
        match value {
            rust::FormulaEngineInitError::EmptyId => Self::EmptyId,
            rust::FormulaEngineInitError::DuplicateId(id) => Self::DuplicateId(id.0),
        }
    }
}

impl From<rust::EngineChangeError> for js::EngineChangeError {
    fn from(value: rust::EngineChangeError) -> Self {
        match value {
            rust::EngineChangeError::EmptyId => Self::EmptyId,
        }
    }
}

impl From<rust::CreateDraftError> for js::CreateDraftError {
    fn from(value: rust::CreateDraftError) -> Self {
        match value {
            rust::CreateDraftError::EmptyId => Self::EmptyId,
        }
    }
}

impl From<rust::ColumnKind> for js::ColumnKind {
    fn from(value: rust::ColumnKind) -> Self {
        match value {
            rust::ColumnKind::Number => Self::Number,
            rust::ColumnKind::String => Self::String,
            rust::ColumnKind::Boolean => Self::Boolean,
            rust::ColumnKind::Date => Self::Date,
            rust::ColumnKind::List => Self::List,
            rust::ColumnKind::Union => Self::Union,
        }
    }
}

impl From<rust::EvaluateInputError> for js::EvaluateInputError {
    fn from(value: rust::EvaluateInputError) -> Self {
        match value {
            rust::EvaluateInputError::InvalidNow { now } => Self::InvalidNow { now },
            rust::EvaluateInputError::InvalidTimeZone { time_zone } => {
                Self::InvalidTimeZone { time_zone }
            }
            rust::EvaluateInputError::EmptyRowId { row_index } => Self::EmptyRowId {
                row_index: row_index as u32,
            },
            rust::EvaluateInputError::DuplicateRowId { id } => Self::DuplicateRowId { id: id.0 },
            rust::EvaluateInputError::EmptyFormulaIds => Self::EmptyFormulaIds,
            rust::EvaluateInputError::InvalidFormulaId { id } => {
                Self::InvalidFormulaId { id: id.0 }
            }
            rust::EvaluateInputError::DuplicateFormulaId { id } => {
                Self::DuplicateFormulaId { id: id.0 }
            }
            rust::EvaluateInputError::MissingInputs { ids } => Self::MissingInputs {
                ids: ids.into_iter().map(|id| id.0).collect(),
            },
            rust::EvaluateInputError::UnexpectedInputs { ids } => Self::UnexpectedInputs {
                ids: ids.into_iter().map(|id| id.0).collect(),
            },
            rust::EvaluateInputError::InvalidColumnType {
                id,
                expected,
                actual,
            } => Self::InvalidColumnType {
                id: id.0,
                expected: expected.into(),
                actual: actual.into(),
            },
            rust::EvaluateInputError::InvalidColumnLength {
                id,
                expected,
                values_len,
                validity_len,
            } => Self::InvalidColumnLength {
                id: id.0,
                expected: expected as u32,
                values_len: values_len as u32,
                validity_len: validity_len as u32,
            },
            rust::EvaluateInputError::InvalidValueType {
                id,
                row_index,
                element_path,
                expected,
                actual,
            } => Self::InvalidValueType {
                id: id.0,
                row_index: row_index as u32,
                element_path: element_path.into_iter().map(|index| index as u32).collect(),
                expected: expected.into(),
                actual: actual.into(),
            },
        }
    }
}

impl From<rust::UpdateExpressionError> for js::UpdateExpressionError {
    fn from(value: rust::UpdateExpressionError) -> Self {
        match value {
            rust::UpdateExpressionError::VersionMismatch => Self::VersionMismatch,
            rust::UpdateExpressionError::InvalidCursor => Self::InvalidCursor,
            rust::UpdateExpressionError::InvalidEditRange => Self::InvalidEditRange,
            rust::UpdateExpressionError::OverlappingEdits => Self::OverlappingEdits,
        }
    }
}

impl Converter {
    pub(crate) fn draft_state(state: &rust::FormulaDraftState) -> js::FormulaDraftState {
        let source = &state.definition.expression;
        js::FormulaDraftState {
            version: state.version.0,
            definition: state.definition.clone().into(),
            output_type: state.output_type.clone().into(),
            diagnostics: state
                .diagnostics
                .iter()
                .map(|d| js::ExpressionDiagnostic {
                    id: d.id.0.clone(),
                    span: span_dto(source, d.span),
                    message: d.message.clone(),
                })
                .collect(),
            tokens: state.tokens.iter().map(|t| token_view(source, t)).collect(),
        }
    }

    pub(crate) fn formula_edit(source: &str, edit: rust::FormulaEdit) -> js::FormulaEdit {
        js::FormulaEdit {
            base_version: edit.base_version.0,
            edits: edit
                .edits
                .into_iter()
                .map(|e| TextEdit {
                    range: span_dto(source, e.range),
                    new_text: e.new_text,
                })
                .collect(),
        }
    }

    pub(crate) fn cursor_help(source: &str, help: rust::CursorHelp) -> js::CursorHelp {
        let output = Self::help_output_view(
            source,
            &ide::HelpResult {
                completion: help.completion,
                signature_help: help.signature_help,
            },
        );
        js::CursorHelp {
            base_version: help.base_version.0,
            completion: js::CompletionResult {
                items: output.completion.items,
                replace: output.completion.replace,
                preferred_indices: output
                    .completion
                    .preferred_indices
                    .into_iter()
                    .map(|index| index as u32)
                    .collect(),
            },
            signature_help: output.signature_help.map(|help| js::SignatureHelp {
                signatures: help.signatures,
                active_signature: help.active_signature as u32,
                active_parameter: help.active_parameter as u32,
            }),
        }
    }

    pub(crate) fn expression_update(
        state: &rust::FormulaDraftState,
        update: js::ExpressionUpdate,
    ) -> Result<rust::ExpressionUpdate, rust::UpdateExpressionError> {
        match update {
            js::ExpressionUpdate::Replace(source) => Ok(rust::ExpressionUpdate::Replace(source)),
            js::ExpressionUpdate::Edits { edit, cursor } => {
                if edit.base_version != state.version.0 {
                    return Err(rust::UpdateExpressionError::VersionMismatch);
                }
                let source = &state.definition.expression;
                let edits = utf16_to_8_text_edits(source, edit.edits).map_err(edit_error)?;
                let cursor = utf16_to_8_cursor(source, cursor).map_err(edit_error)?;
                Ok(rust::ExpressionUpdate::Edits {
                    edit: rust::FormulaEdit {
                        base_version: rust::DraftVersion(edit.base_version),
                        edits,
                    },
                    cursor: rust::TextOffset(cursor),
                })
            }
        }
    }
}

fn edit_error(error: ide::IdeError) -> rust::UpdateExpressionError {
    match error {
        ide::IdeError::InvalidCursor => rust::UpdateExpressionError::InvalidCursor,
        ide::IdeError::InvalidEditRange => rust::UpdateExpressionError::InvalidEditRange,
        ide::IdeError::OverlappingEdits => rust::UpdateExpressionError::OverlappingEdits,
        ide::IdeError::FormatError => unreachable!("coordinate conversion does not format"),
    }
}
