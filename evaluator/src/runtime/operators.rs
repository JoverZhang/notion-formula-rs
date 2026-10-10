use analyzer::analysis::Ty;
use analyzer::ast::{BinOpKind, UnOp};

use crate::builtins::{RowOutcome, rows_to_kernel};
use crate::core::columns::{
    AbiKind, AnyKind, BooleanKind, Column, ColumnKind, DateKind, KernelColumn, ListKind,
    NumberKind, TextKind, Validity,
};
use crate::core::errors::EvalError;
use crate::core::types::{EvalBlock, Mask, Value, value_type_accepts, values_equal};

pub(crate) fn literal_block(value: Value, mask: &Mask) -> EvalBlock {
    let len = mask.len();
    let column = match value {
        Value::Number(value) => Column::Number(KernelColumn::from_values(
            vec![value; len],
            Validity::AllValid,
        )),
        Value::Text(value) => Column::Text(KernelColumn::from_values(
            vec![value; len],
            Validity::AllValid,
        )),
        Value::Bool(value) => Column::Boolean(KernelColumn::from_values(
            vec![value; len],
            Validity::AllValid,
        )),
        Value::Date(value) => Column::Date(KernelColumn::from_values(
            vec![value.into(); len],
            Validity::AllValid,
        )),
        Value::DateValue(value) => Column::Date(KernelColumn::from_values(
            vec![value; len],
            Validity::AllValid,
        )),
        Value::List(value) => Column::List(KernelColumn::from_values(
            vec![value; len],
            Validity::AllValid,
        )),
    };
    EvalBlock::new(column, Mask::all(mask.len()), Vec::new())
}

pub(crate) fn eval_cast(input: EvalBlock, target: AbiKind, mask: &Mask) -> EvalBlock {
    if input.column.abi_kind() == target {
        return EvalBlock::new(
            input.column.normalize_inactive(mask),
            input.ok,
            input.errors,
        );
    }
    match target {
        AbiKind::Number => cast_rows::<NumberKind>(input, mask),
        AbiKind::Boolean => cast_rows::<BooleanKind>(input, mask),
        AbiKind::Text => cast_rows::<TextKind>(input, mask),
        AbiKind::Date => cast_rows::<DateKind>(input, mask),
        AbiKind::List => cast_rows::<ListKind>(input, mask),
        AbiKind::Any => cast_rows::<AnyKind>(input, mask),
    }
}

pub(crate) fn eval_type_check(mut input: EvalBlock, expected: &Ty, mask: &Mask) -> EvalBlock {
    for row in 0..mask.len() {
        if !mask[row] || !input.ok[row] {
            continue;
        }
        let Some(value) = input.column.row_value(row) else {
            continue;
        };
        let actual = value.value_type();
        if !value_type_accepts(expected, &actual) {
            input.ok.set(row, false);
            input.errors.push((
                row,
                EvalError::InvalidValueType {
                    expected: expected.clone(),
                    actual,
                },
            ));
        }
    }
    input
}

fn cast_rows<K: ColumnKind>(input: EvalBlock, mask: &Mask) -> EvalBlock {
    let rows = (0..mask.len())
        .map(|row| {
            if !mask[row] {
                RowOutcome::Inactive
            } else if !input.ok[row] {
                RowOutcome::Failed
            } else {
                input
                    .column
                    .row_value(row)
                    .map(RowOutcome::Value)
                    .unwrap_or(RowOutcome::Null)
            }
        })
        .collect();
    let mut output = rows_to_kernel::<K>(rows, mask).into_eval_block();
    output.errors.extend(input.errors);
    output
}

pub(crate) fn eval_list(blocks: Vec<EvalBlock>, mask: &Mask) -> EvalBlock {
    let mut errors = Vec::new();
    for block in &blocks {
        errors.extend(block.errors.iter().cloned());
    }
    let rows = (0..mask.len())
        .map(|row| {
            if !mask[row] {
                return RowOutcome::Inactive;
            }
            if blocks.iter().any(|block| !block.ok[row]) {
                return RowOutcome::Failed;
            }
            let mut values = Vec::with_capacity(blocks.len());
            for block in &blocks {
                values.push(block.column.row_value(row));
            }
            RowOutcome::Value(Value::List(values))
        })
        .collect();
    let mut result = rows_to_kernel::<crate::core::columns::ListKind>(rows, mask).into_eval_block();
    result.errors.extend(errors);
    result
}

pub(crate) fn eval_unary(op: UnOp, input: EvalBlock, mask: &Mask) -> EvalBlock {
    let rows = (0..mask.len())
        .map(|row| {
            if !mask[row] {
                return RowOutcome::Inactive;
            }
            if !input.ok[row] {
                return RowOutcome::Failed;
            }
            let Some(value) = input.column.row_value(row) else {
                return RowOutcome::Null;
            };
            match (op, value) {
                (UnOp::Neg, Value::Number(value)) => RowOutcome::Value(Value::Number(-value)),
                (UnOp::Not(_), Value::Bool(value)) => RowOutcome::Value(Value::Bool(!value)),
                (UnOp::Neg, value) => {
                    RowOutcome::Error(EvalError::invalid_type(Ty::Number, &value))
                }
                (UnOp::Not(_), value) => {
                    RowOutcome::Error(EvalError::invalid_type(Ty::Boolean, &value))
                }
            }
        })
        .collect();
    let mut result = rows_to_kernel::<AnyKind>(rows, mask).into_eval_block();
    result.errors.extend(input.errors);
    result
}

pub(crate) fn eval_binary(
    op: BinOpKind,
    left: EvalBlock,
    right: EvalBlock,
    mask: &Mask,
) -> EvalBlock {
    let rows = (0..mask.len())
        .map(|row| {
            if !mask[row] {
                return RowOutcome::Inactive;
            }
            if !left.ok[row] || !right.ok[row] {
                return RowOutcome::Failed;
            }
            let (Some(left), Some(right)) =
                (left.column.row_value(row), right.column.row_value(row))
            else {
                return RowOutcome::Null;
            };
            eval_binary_row(op, left, right)
        })
        .collect();
    let mut result = rows_to_kernel::<AnyKind>(rows, mask).into_eval_block();
    result.errors.extend(left.errors);
    result.errors.extend(right.errors);
    result
}

fn eval_binary_row(op: BinOpKind, left: Value, right: Value) -> RowOutcome {
    use BinOpKind::*;
    match (op, left, right) {
        (Plus, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(left + right))
        }
        (Plus, Value::Text(left), right) => {
            RowOutcome::Value(Value::Text(left + &stringify_value(&right)))
        }
        (Plus, left, Value::Text(right)) => {
            RowOutcome::Value(Value::Text(stringify_value(&left) + &right))
        }
        (Minus, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(left - right))
        }
        (Star, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(left * right))
        }
        (Slash, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(left / right))
        }
        (Percent, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(left % right))
        }
        (Caret, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(pow_number(left, right)))
        }
        (EqEq, left, right) => RowOutcome::Value(Value::Bool(values_equal(&left, &right))),
        (Ne, left, right) => RowOutcome::Value(Value::Bool(!values_equal(&left, &right))),
        (Lt | Le | Ge | Gt, Value::Number(left), Value::Number(right)) => {
            let matches = match op {
                Lt => left < right,
                Le => left <= right,
                Ge => left >= right,
                Gt => left > right,
                _ => unreachable!(),
            };
            RowOutcome::Value(Value::Bool(matches))
        }
        (Lt | Le | Ge | Gt, left, right) => compare_values(&left, &right)
            .map(|ordering| {
                let matches = match op {
                    Lt => ordering.is_lt(),
                    Le => ordering.is_le(),
                    Ge => ordering.is_ge(),
                    Gt => ordering.is_gt(),
                    _ => false,
                };
                RowOutcome::Value(Value::Bool(matches))
            })
            .unwrap_or_else(|| RowOutcome::Error(comparison_error(&left, &right))),
        (_, left, right) => {
            let actual = if matches!(left, Value::Number(_)) {
                &right
            } else {
                &left
            };
            RowOutcome::Error(EvalError::invalid_type(Ty::Number, actual))
        }
    }
}

fn comparison_error(left: &Value, right: &Value) -> EvalError {
    let comparable = Ty::Union(vec![Ty::Boolean, Ty::Number, Ty::String, Ty::Date]);
    if matches!(left, Value::List(_)) {
        EvalError::invalid_type(comparable, left)
    } else {
        EvalError::invalid_type(left.value_type(), right)
    }
}

pub(crate) fn pow_number(base: f64, exponent: f64) -> f64 {
    if base.abs() == 1.0 && !exponent.is_finite() {
        f64::NAN
    } else {
        base.powf(exponent)
    }
}

fn compare_values(left: &Value, right: &Value) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.partial_cmp(right),
        (Value::Text(left), Value::Text(right)) => Some(left.cmp(right)),
        (Value::Bool(left), Value::Bool(right)) => Some(left.cmp(right)),
        (Value::Date(left), Value::Date(right)) => Some(left.cmp(right)),
        (Value::DateValue(left), Value::DateValue(right)) => Some(left.start.cmp(&right.start)),
        (Value::Date(left), Value::DateValue(right)) => Some(left.cmp(&right.start)),
        (Value::DateValue(left), Value::Date(right)) => Some(left.start.cmp(right)),
        _ => None,
    }
}

pub(crate) fn eval_logical_and(
    left: EvalBlock,
    mask: &Mask,
    evaluate_right: impl FnOnce(&Mask) -> EvalBlock,
) -> EvalBlock {
    eval_logical(LogicalMode::And, left, mask, evaluate_right)
}

pub(crate) fn eval_logical_or(
    left: EvalBlock,
    mask: &Mask,
    evaluate_right: impl FnOnce(&Mask) -> EvalBlock,
) -> EvalBlock {
    eval_logical(LogicalMode::Or, left, mask, evaluate_right)
}

#[derive(Clone, Copy)]
enum LogicalMode {
    And,
    Or,
}

impl LogicalMode {
    fn evaluates_right(self, left: Option<Value>) -> bool {
        match self {
            Self::And => matches!(left, Some(Value::Bool(true))),
            Self::Or => matches!(left, None | Some(Value::Bool(false))),
        }
    }

    fn short_circuits(self, left: bool) -> bool {
        match self {
            Self::And => !left,
            Self::Or => left,
        }
    }
}

fn eval_logical(
    mode: LogicalMode,
    left: EvalBlock,
    mask: &Mask,
    evaluate_right: impl FnOnce(&Mask) -> EvalBlock,
) -> EvalBlock {
    let mut right_mask = Mask::none(mask.len());
    for row in 0..mask.len() {
        if mask[row] && left.ok[row] && mode.evaluates_right(left.column.row_value(row)) {
            right_mask.set(row, true);
        }
    }
    let right = evaluate_right(&right_mask);
    merge_logical(mode, left, right, mask)
}

fn merge_logical(mode: LogicalMode, left: EvalBlock, right: EvalBlock, mask: &Mask) -> EvalBlock {
    let rows = (0..mask.len())
        .map(|row| {
            if !mask[row] {
                return RowOutcome::Inactive;
            }
            if !left.ok[row] {
                return RowOutcome::Failed;
            }
            let left_value = match left.column.row_value(row) {
                Some(Value::Bool(value)) => value,
                None => false,
                Some(value) => {
                    return RowOutcome::Error(EvalError::invalid_type(Ty::Boolean, &value));
                }
            };
            if mode.short_circuits(left_value) {
                return RowOutcome::Value(Value::Bool(left_value));
            }
            if !right.ok[row] {
                return RowOutcome::Failed;
            }
            match right.column.row_value(row) {
                Some(Value::Bool(value)) => RowOutcome::Value(Value::Bool(value)),
                None => RowOutcome::Null,
                Some(value) => RowOutcome::Error(EvalError::invalid_type(Ty::Boolean, &value)),
            }
        })
        .collect();
    let mut result = rows_to_kernel::<AnyKind>(rows, mask).into_eval_block();
    result.errors.extend(left.errors);
    result.errors.extend(right.errors);
    result
}

pub(crate) fn split_condition(condition: &EvalBlock, mask: &Mask) -> (Mask, Mask) {
    let mut truthy = Mask::none(mask.len());
    let mut falsy = Mask::none(mask.len());
    for row in 0..mask.len() {
        if !mask[row] || !condition.ok[row] {
            continue;
        }
        match condition.column.row_value(row) {
            Some(Value::Bool(true)) => truthy.set(row, true),
            Some(Value::Bool(false)) | None => falsy.set(row, true),
            Some(_) => {}
        }
    }
    (truthy, falsy)
}

pub(crate) fn merge_condition(
    condition: EvalBlock,
    then_block: EvalBlock,
    else_block: EvalBlock,
    mask: &Mask,
    then_mask: &Mask,
) -> EvalBlock {
    let rows = (0..mask.len())
        .map(|row| {
            if !mask[row] {
                return RowOutcome::Inactive;
            }
            if !condition.ok[row] {
                return RowOutcome::Failed;
            }
            if let Some(value) = condition.column.row_value(row)
                && !matches!(value, Value::Bool(_))
            {
                return RowOutcome::Error(EvalError::invalid_type(Ty::Boolean, &value));
            }
            let selected = if then_mask[row] {
                &then_block
            } else {
                &else_block
            };
            if !selected.ok[row] {
                return RowOutcome::Failed;
            }
            selected
                .column
                .row_value(row)
                .map(RowOutcome::Value)
                .unwrap_or(RowOutcome::Null)
        })
        .collect();
    let mut result = rows_to_kernel::<AnyKind>(rows, mask).into_eval_block();
    result.errors.extend(condition.errors);
    result.errors.extend(then_block.errors);
    result.errors.extend(else_block.errors);
    result
}

pub(crate) fn stringify_value(value: &Value) -> String {
    match value {
        Value::Number(value) => {
            if value.is_nan() {
                "NaN".to_string()
            } else if *value == f64::INFINITY {
                "Infinity".to_string()
            } else if *value == f64::NEG_INFINITY {
                "-Infinity".to_string()
            } else if *value == 0.0 {
                "0".to_string()
            } else if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        Value::Text(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Date(value) => value.to_string(),
        Value::DateValue(value) => match value.end {
            Some(end) => format!("{} → {end}", value.start),
            None => value.start.to_string(),
        },
        Value::List(values) => {
            let values = values
                .iter()
                .map(|value| value.as_ref().map_or(String::new(), stringify_value))
                .collect::<Vec<_>>();
            format!("[{}]", values.join(", "))
        }
    }
}
