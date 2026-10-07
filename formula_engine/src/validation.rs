use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Column, ColumnKind, EvaluateInput, EvaluateInputError, PropertyDefinition, RuntimeContext,
    Value, ValueType,
};

/// Runtime snapshot whose UTC and offset-adjusted local times are valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ValidatedRuntimeContext {
    pub now: i64,
    pub time_zone_offset_seconds: i32,
}

/// Validate a request against the Engine's already-validated definitions.
///
/// Selection is deterministic: runtime, rows in request order, requested formulas
/// in request order, missing and extra input sets, then columns ordered by ID.
/// Each column checks its physical kind, lengths, and present rows in that order;
/// nested values are visited in list order. Formula readiness is not a request
/// constraint, and all Input definitions participate regardless of dependencies.
pub(crate) fn validate_evaluate_input<'a>(
    definitions: impl IntoIterator<Item = &'a PropertyDefinition>,
    input: &EvaluateInput,
) -> Result<ValidatedRuntimeContext, EvaluateInputError> {
    let runtime = validate_runtime(&input.runtime)?;

    let mut row_ids = BTreeSet::new();
    for (row_index, id) in input.row_ids.iter().enumerate() {
        if id.0.is_empty() {
            return Err(EvaluateInputError::EmptyRowId { row_index });
        }
        if !row_ids.insert(id) {
            return Err(EvaluateInputError::DuplicateRowId { id: id.clone() });
        }
    }

    let mut inputs = BTreeMap::new();
    let mut formulas = BTreeSet::new();
    for definition in definitions {
        match definition {
            PropertyDefinition::Input { id, ty } => {
                inputs.insert(id, ty);
            }
            PropertyDefinition::Formula(formula) => {
                formulas.insert(&formula.id);
            }
        }
    }

    if input.formula_ids.is_empty() {
        return Err(EvaluateInputError::EmptyFormulaIds);
    }
    let mut requested = BTreeSet::new();
    for id in &input.formula_ids {
        if id.0.is_empty() || !formulas.contains(id) {
            return Err(EvaluateInputError::InvalidFormulaId { id: id.clone() });
        }
        if !requested.insert(id) {
            return Err(EvaluateInputError::DuplicateFormulaId { id: id.clone() });
        }
    }

    let expected_ids: BTreeSet<_> = inputs.keys().copied().collect();
    let provided_ids: BTreeSet<_> = input.columns.keys().collect();
    let missing: Vec<_> = expected_ids
        .difference(&provided_ids)
        .copied()
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(EvaluateInputError::MissingInputs { ids: missing });
    }
    let unexpected: Vec<_> = provided_ids
        .difference(&expected_ids)
        .copied()
        .cloned()
        .collect();
    if !unexpected.is_empty() {
        return Err(EvaluateInputError::UnexpectedInputs { ids: unexpected });
    }

    for (id, ty) in inputs {
        let column = &input.columns[id];
        let expected = column_kind(ty);
        let (actual, values_len, validity_len) = column_shape(column);
        if actual != expected {
            return Err(EvaluateInputError::InvalidColumnType {
                id: id.clone(),
                expected,
                actual,
            });
        }
        if values_len != input.row_ids.len() || validity_len != input.row_ids.len() {
            return Err(EvaluateInputError::InvalidColumnLength {
                id: id.clone(),
                expected: input.row_ids.len(),
                values_len,
                validity_len,
            });
        }

        let failure = match (ty, column) {
            (ValueType::List(element_type), Column::List(data)) => {
                data.validity.valid_indices().find_map(|row_index| {
                    list_mismatch(element_type, &data.values[row_index], &mut Vec::new())
                        .map(|mismatch| (row_index, mismatch))
                })
            }
            (_, Column::Union(data)) => data.validity.valid_indices().find_map(|row_index| {
                value_mismatch(ty, &data.values[row_index], &mut Vec::new())
                    .map(|mismatch| (row_index, mismatch))
            }),
            // A scalar column's physical kind already establishes its type.
            // Number values are unrestricted binary64; Date inputs have no range
            // constraint until a date operation uses them.
            _ => None,
        };
        if let Some((row_index, mismatch)) = failure {
            return Err(EvaluateInputError::InvalidValueType {
                id: id.clone(),
                row_index,
                element_path: mismatch.path,
                expected: mismatch.expected,
                actual: mismatch.actual,
            });
        }
    }

    Ok(runtime)
}

fn validate_runtime(
    runtime: &RuntimeContext,
) -> Result<ValidatedRuntimeContext, EvaluateInputError> {
    // Gregorian 0001-01-01T00:00:00.000 through 9999-12-31T23:59:59.999,
    // expressed as UTC Unix milliseconds; offset time uses the same boundaries.
    const MIN_NOW: i64 = -62_135_596_800_000;
    const MAX_NOW: i64 = 253_402_300_799_999;
    let valid_time = |timestamp| (MIN_NOW..=MAX_NOW).contains(&timestamp);
    if !valid_time(runtime.now) {
        return Err(EvaluateInputError::InvalidNow { now: runtime.now });
    }
    let offset =
        parse_time_zone(&runtime.time_zone).ok_or_else(|| EvaluateInputError::InvalidTimeZone {
            time_zone: runtime.time_zone.clone(),
        })?;
    if !runtime
        .now
        .checked_add(i64::from(offset) * 1_000)
        .is_some_and(valid_time)
    {
        return Err(EvaluateInputError::InvalidNow { now: runtime.now });
    }
    Ok(ValidatedRuntimeContext {
        now: runtime.now,
        time_zone_offset_seconds: offset,
    })
}

fn parse_time_zone(time_zone: &str) -> Option<i32> {
    let [sign, hour_tens, hour_units, b':', minute_tens, minute_units] = time_zone.as_bytes()
    else {
        return None;
    };
    if !matches!(sign, b'+' | b'-')
        || ![hour_tens, hour_units, minute_tens, minute_units]
            .into_iter()
            .all(|digit| digit.is_ascii_digit())
    {
        return None;
    }
    let hours = (hour_tens - b'0') * 10 + (hour_units - b'0');
    let minutes = (minute_tens - b'0') * 10 + (minute_units - b'0');
    if hours > 23 || minutes > 59 {
        return None;
    }
    let seconds = i32::from(hours) * 3_600 + i32::from(minutes) * 60;
    Some(if *sign == b'-' { -seconds } else { seconds })
}

fn column_kind(ty: &ValueType) -> ColumnKind {
    match ty {
        ValueType::Number => ColumnKind::Number,
        ValueType::String => ColumnKind::String,
        ValueType::Boolean => ColumnKind::Boolean,
        ValueType::Date => ColumnKind::Date,
        ValueType::List(_) => ColumnKind::List,
        ValueType::Union(_) | ValueType::Unknown => ColumnKind::Union,
    }
}

fn column_shape(column: &Column) -> (ColumnKind, usize, usize) {
    match column {
        Column::Number(data) => (ColumnKind::Number, data.values.len(), data.validity.len()),
        Column::String(data) => (ColumnKind::String, data.values.len(), data.validity.len()),
        Column::Boolean(data) => (ColumnKind::Boolean, data.values.len(), data.validity.len()),
        Column::Date(data) => (ColumnKind::Date, data.values.len(), data.validity.len()),
        Column::DateValue(data) => (ColumnKind::Date, data.values.len(), data.validity.len()),
        Column::List(data) => (ColumnKind::List, data.values.len(), data.validity.len()),
        Column::Union(data) => (ColumnKind::Union, data.values.len(), data.validity.len()),
    }
}

struct ValueMismatch {
    path: Vec<usize>,
    expected: ValueType,
    actual: ValueType,
}

fn list_mismatch(
    element_type: &ValueType,
    items: &[Option<Value>],
    path: &mut Vec<usize>,
) -> Option<ValueMismatch> {
    for (index, item) in items.iter().enumerate() {
        let Some(value) = item else { continue };
        path.push(index);
        let mismatch = value_mismatch(element_type, value, path);
        path.pop();
        if mismatch.is_some() {
            return mismatch;
        }
    }
    None
}

fn value_mismatch(ty: &ValueType, value: &Value, path: &mut Vec<usize>) -> Option<ValueMismatch> {
    match (ty, value) {
        (ValueType::Unknown, _)
        | (ValueType::Number, Value::Number(_))
        | (ValueType::String, Value::String(_))
        | (ValueType::Boolean, Value::Boolean(_))
        | (ValueType::Date, Value::Date(_) | Value::DateValue(_)) => None,
        (ValueType::List(element_type), Value::List(items)) => {
            list_mismatch(element_type, items, path)
        }
        (ValueType::Union(members), _) if members.iter().any(|member| accepts(member, value)) => {
            None
        }
        // A failed Union reports its own position, never a losing member's
        // nested mismatch. The actual type is inferred from the complete value.
        _ => Some(ValueMismatch {
            path: path.clone(),
            expected: ty.clone(),
            actual: actual_type(value),
        }),
    }
}

fn accepts(ty: &ValueType, value: &Value) -> bool {
    match (ty, value) {
        (ValueType::Unknown, _)
        | (ValueType::Number, Value::Number(_))
        | (ValueType::String, Value::String(_))
        | (ValueType::Boolean, Value::Boolean(_))
        | (ValueType::Date, Value::Date(_) | Value::DateValue(_)) => true,
        (ValueType::List(element_type), Value::List(items)) => items.iter().all(|item| {
            item.as_ref()
                .is_none_or(|value| accepts(element_type, value))
        }),
        (ValueType::Union(members), _) => members.iter().any(|member| accepts(member, value)),
        _ => false,
    }
}

fn actual_type(value: &Value) -> ValueType {
    match value {
        Value::Number(_) => ValueType::Number,
        Value::String(_) => ValueType::String,
        Value::Boolean(_) => ValueType::Boolean,
        Value::Date(_) | Value::DateValue(_) => ValueType::Date,
        Value::List(items) => {
            // Null elements contribute no type. Preserve first occurrence order
            // while deduplicating the concrete structures of present elements.
            let mut members = Vec::new();
            for item in items.iter().flatten() {
                let ty = actual_type(item);
                if !members.contains(&ty) {
                    members.push(ty);
                }
            }
            let element_type = match members.len() {
                0 => ValueType::Unknown,
                1 => members.pop().expect("one member exists"),
                _ => ValueType::Union(members),
            };
            ValueType::List(Box::new(element_type))
        }
    }
}

#[cfg(test)]
mod tests;
