use std::collections::{BTreeMap, BTreeSet, HashMap};

use analyzer::analysis::Ty;
use evaluator::{
    AbiKind, AnyKind, BooleanKind, BuiltinRuntimeContext, Column as RuntimeColumn, DateKind,
    EvalBlock, EvalError, EvalInputsBuilder, KernelColumn, ListKind, Mask, NumberKind, RowBatch,
    TextKind, Validity, Value as RuntimeValue,
};

use super::{
    FormulaEngineInner, FormulaStatus, PropertyDefinition, PropertyId, from_analyzer_type,
};
use crate::validation::validate_evaluate_input;
use crate::{
    Column, ColumnData, EvaluateInput, EvaluateInputError, EvaluateResult, FormulaEvaluationError,
    FormulaOutput, NullBuffer, RowError, RuntimeError, Value, ValueType,
};

pub(super) fn evaluate(
    engine: &FormulaEngineInner,
    input: &EvaluateInput,
) -> Result<EvaluateResult, EvaluateInputError> {
    let validated = validate_evaluate_input(engine.definitions.values(), input)?;
    let runtime =
        BuiltinRuntimeContext::new(validated.now, validated.time_zone_offset_seconds / 60);
    let batch = RowBatch::new(input.row_ids.iter().map(|row| row.0.clone().into()), 0);
    let mut values = BTreeMap::new();
    for (id, definition) in &engine.definitions {
        if let PropertyDefinition::Input { ty, .. } = definition {
            let rows = input_values(&input.columns[id]);
            let column = runtime_column(rows, runtime_kind(&super::to_analyzer_type(ty)));
            values.insert(
                id.clone(),
                EvalBlock::new(column, Mask::all(batch.len()), vec![]),
            );
        }
    }

    // The graph selects what must be computed. A dependency's errors are read by the
    // evaluator under its execution mask, so static reachability does not propagate them.
    for id in evaluation_order(engine, &input.formula_ids) {
        let prepared = &engine.prepared[&id];
        let mut builder = EvalInputsBuilder::new(runtime.clone());
        for requirement in prepared.required_columns() {
            let dependency = PropertyId(requirement.name.clone());
            builder.insert_block(requirement.slot, values[&dependency].clone());
        }
        let inputs = builder
            .finish(prepared, batch.len())
            .expect("validated values satisfy the cached formula's input layout");
        let mut block = prepared
            .evaluate(batch.clone(), inputs)
            .expect("batch and inputs belong to the cached formula");
        originate_errors(&id, &mut block);
        // Generic kernels use Any storage. Dependents use the statically resolved
        // type, including for zero rows and all-null results.
        block.column = runtime_column(block_values(&block), runtime_kind(prepared.output_type()));
        values.insert(id, block);
    }

    let formulas = input
        .formula_ids
        .iter()
        .map(|id| {
            let result = match &engine.statuses[id] {
                FormulaStatus::NotReady => Err(FormulaEvaluationError::NotReady),
                FormulaStatus::Ready { output_type } => {
                    let block = &values[id];
                    Ok(FormulaOutput {
                        output_type: output_type.clone(),
                        column: output_column(block, output_type),
                        errors: block.errors.iter().map(public_error).collect(),
                    })
                }
            };
            (id.clone(), result)
        })
        .collect::<HashMap<_, _>>();
    Ok(EvaluateResult { formulas })
}

fn evaluation_order(engine: &FormulaEngineInner, requested: &[PropertyId]) -> Vec<PropertyId> {
    let roots = requested.iter().collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for root in roots {
        if !matches!(engine.statuses[root], FormulaStatus::Ready { .. }) {
            continue;
        }
        let mut pending = vec![(root.clone(), false)];
        while let Some((id, complete)) = pending.pop() {
            if complete {
                order.push(id);
                continue;
            }
            if !visited.insert(id.clone()) {
                continue;
            }
            pending.push((id.clone(), true));
            for dependency in engine.dependencies[&id].iter().rev() {
                if matches!(
                    engine.definitions[dependency],
                    PropertyDefinition::Formula(_)
                ) {
                    pending.push((dependency.clone(), false));
                }
            }
        }
    }
    order
}

fn originate_errors(id: &PropertyId, block: &mut EvalBlock) {
    let mut seen = BTreeSet::new();
    let mut occurrence = 0;
    block.errors = std::mem::take(&mut block.errors)
        .into_iter()
        .filter_map(|(row, error)| {
            let error = match error {
                EvalError::Originated { .. } => error,
                error => {
                    let failure = EvalError::Originated {
                        origin_formula_id: id.0.clone(),
                        occurrence,
                        error: Box::new(error),
                    };
                    occurrence += 1;
                    failure
                }
            };
            let EvalError::Originated {
                origin_formula_id,
                occurrence,
                ..
            } = &error
            else {
                unreachable!();
            };
            seen.insert((row, origin_formula_id.clone(), *occurrence))
                .then_some((row, error))
        })
        .collect();
}

fn public_error((row_index, error): &(usize, EvalError)) -> RowError {
    let EvalError::Originated {
        origin_formula_id,
        error,
        ..
    } = error
    else {
        unreachable!("formula errors are assigned origins before they are cached");
    };
    let error = match error.as_ref() {
        EvalError::InvalidValueType { expected, actual } => RuntimeError::InvalidValueType {
            expected: from_analyzer_type(expected),
            actual: from_analyzer_type(actual),
        },
        EvalError::InvalidValue { actual, constraint } => RuntimeError::InvalidValue {
            actual: public_value(actual.clone()),
            constraint: constraint.clone(),
        },
        EvalError::InvalidRegex { pattern, detail } => RuntimeError::InvalidRegex {
            pattern: pattern.clone(),
            detail: detail.clone(),
        },
        EvalError::InvalidDateText { text } => RuntimeError::InvalidDateText { text: text.clone() },
        EvalError::DateOutOfRange => RuntimeError::DateOutOfRange,
        other => unreachable!("prepared formulas expose only specified runtime errors: {other:?}"),
    };
    RowError {
        row_index: *row_index,
        origin_formula_id: origin_formula_id.clone().into(),
        error,
    }
}

fn input_values(column: &Column) -> Vec<Option<RuntimeValue>> {
    macro_rules! rows {
        ($data:expr, $convert:expr) => {
            $data
                .values
                .iter()
                .enumerate()
                .map(|(row, value)| {
                    $data
                        .validity
                        .is_valid(row)
                        .then(|| ($convert)(value.clone()))
                })
                .collect()
        };
    }
    match column {
        Column::Number(data) => rows!(data, RuntimeValue::Number),
        Column::String(data) => rows!(data, RuntimeValue::Text),
        Column::Boolean(data) => rows!(data, RuntimeValue::Bool),
        Column::Date(data) => rows!(data, RuntimeValue::Date),
        Column::List(data) => rows!(data, |items: Vec<Option<Value>>| RuntimeValue::List(
            items
                .into_iter()
                .map(|value| value.map(runtime_value))
                .collect()
        )),
        Column::Union(data) => rows!(data, runtime_value),
    }
}

fn runtime_value(value: Value) -> RuntimeValue {
    match value {
        Value::Number(value) => RuntimeValue::Number(value),
        Value::String(value) => RuntimeValue::Text(value),
        Value::Boolean(value) => RuntimeValue::Bool(value),
        Value::Date(value) => RuntimeValue::Date(value),
        Value::List(values) => RuntimeValue::List(
            values
                .into_iter()
                .map(|value| value.map(runtime_value))
                .collect(),
        ),
    }
}

fn public_value(value: RuntimeValue) -> Value {
    match value {
        RuntimeValue::Number(value) => Value::Number(value),
        RuntimeValue::Text(value) => Value::String(value),
        RuntimeValue::Bool(value) => Value::Boolean(value),
        RuntimeValue::Date(value) => Value::Date(value),
        RuntimeValue::List(values) => Value::List(
            values
                .into_iter()
                .map(|value| value.map(public_value))
                .collect(),
        ),
    }
}

fn block_values(block: &EvalBlock) -> Vec<Option<RuntimeValue>> {
    (0..block.len())
        .map(|row| {
            if block.ok[row] {
                block.column.row_value(row)
            } else {
                None
            }
        })
        .collect()
}

fn runtime_kind(ty: &Ty) -> AbiKind {
    match ty {
        Ty::Number => AbiKind::Number,
        Ty::String => AbiKind::Text,
        Ty::Boolean => AbiKind::Boolean,
        Ty::Date => AbiKind::Date,
        Ty::List(_) => AbiKind::List,
        Ty::Union(members) => {
            let mut kinds = members
                .iter()
                .filter(|ty| !matches!(ty, Ty::Null))
                .map(runtime_kind);
            let Some(first) = kinds.next() else {
                return AbiKind::Any;
            };
            if kinds.all(|kind| kind == first) {
                first
            } else {
                AbiKind::Any
            }
        }
        _ => AbiKind::Any,
    }
}

fn runtime_column(rows: Vec<Option<RuntimeValue>>, kind: AbiKind) -> RuntimeColumn {
    let validity = Validity::from_valid_bits(rows.iter().map(Option::is_some).collect());
    macro_rules! typed {
        ($kind:ty, $variant:ident, $placeholder:expr) => {{
            let values = rows
                .into_iter()
                .map(|value| match value {
                    Some(RuntimeValue::$variant(value)) => value,
                    None => $placeholder,
                    Some(other) => {
                        unreachable!("resolved storage {kind:?} cannot contain {other:?}")
                    }
                })
                .collect();
            KernelColumn::<$kind>::from_values(values, validity).into_column()
        }};
    }
    match kind {
        AbiKind::Number => typed!(NumberKind, Number, 0.0),
        AbiKind::Text => typed!(TextKind, Text, String::new()),
        AbiKind::Boolean => typed!(BooleanKind, Bool, false),
        AbiKind::Date => typed!(DateKind, Date, 0),
        AbiKind::List => typed!(ListKind, List, Vec::new()),
        AbiKind::Any => KernelColumn::<AnyKind>::from_values(
            rows.into_iter()
                .map(|value| value.unwrap_or(RuntimeValue::Number(0.0)))
                .collect(),
            validity,
        )
        .into_column(),
    }
}

fn output_column(block: &EvalBlock, ty: &ValueType) -> Column {
    let rows = block_values(block)
        .into_iter()
        .map(|value| value.map(public_value))
        .collect::<Vec<_>>();
    let validity = NullBuffer::from(rows.iter().map(Option::is_some).collect::<Vec<_>>());
    macro_rules! typed {
        ($variant:ident, $placeholder:expr) => {{
            let values = rows
                .into_iter()
                .map(|value| match value {
                    Some(Value::$variant(value)) => value,
                    None => $placeholder,
                    Some(other) => unreachable!("resolved output {ty:?} cannot contain {other:?}"),
                })
                .collect();
            Column::$variant(ColumnData { values, validity })
        }};
    }
    match ty {
        ValueType::Number => typed!(Number, 0.0),
        ValueType::String => typed!(String, String::new()),
        ValueType::Boolean => typed!(Boolean, false),
        ValueType::Date => typed!(Date, 0),
        ValueType::List(_) => typed!(List, Vec::new()),
        ValueType::Unknown | ValueType::Union(_) => Column::Union(ColumnData {
            values: rows
                .into_iter()
                .map(|value| value.unwrap_or(Value::Number(0.0)))
                .collect(),
            validity,
        }),
    }
}
