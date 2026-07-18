use analyzer::ast::{BinOpKind, UnOp};

use crate::builtins::{RowOutcome, rows_to_kernel};
use crate::core::columns::{
    AbiKind, AnyKind, BooleanKind, Column, ColumnKind, DateKind, KernelColumn, ListKind,
    NumberKind, TextKind, Validity,
};
use crate::core::errors::EvalError;
use crate::core::types::{EvalBlock, Mask, Value};

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
                let Some(value) = block.column.row_value(row) else {
                    return RowOutcome::Null;
                };
                values.push(value);
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
                _ => RowOutcome::Error(EvalError::TypeMismatch),
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
        (Slash, Value::Number(_), Value::Number(0.0)) => RowOutcome::Error(EvalError::DivideByZero),
        (Slash, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(left / right))
        }
        (Percent, Value::Number(_), Value::Number(0.0)) => {
            RowOutcome::Error(EvalError::DivideByZero)
        }
        (Percent, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(left % right))
        }
        (Caret, Value::Number(left), Value::Number(right)) => {
            RowOutcome::Value(Value::Number(left.powf(right)))
        }
        (EqEq, left, right) => RowOutcome::Value(Value::Bool(left == right)),
        (Ne, left, right) => RowOutcome::Value(Value::Bool(left != right)),
        (Lt | Le | Ge | Gt, left, right) => compare_values(op, &left, &right)
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
            .unwrap_or(RowOutcome::Error(EvalError::TypeMismatch)),
        _ => RowOutcome::Error(EvalError::TypeMismatch),
    }
}

fn compare_values(left_op: BinOpKind, left: &Value, right: &Value) -> Option<std::cmp::Ordering> {
    let _ = left_op;
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.partial_cmp(right),
        (Value::Text(left), Value::Text(right)) => Some(left.cmp(right)),
        (Value::Bool(left), Value::Bool(right)) => Some(left.cmp(right)),
        (Value::Date(left), Value::Date(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

pub(crate) fn eval_logical_and(
    left: EvalBlock,
    mask: &Mask,
    evaluate_right: impl FnOnce(&Mask) -> EvalBlock,
) -> EvalBlock {
    let mut right_mask = Mask::none(mask.len());
    for row in 0..mask.len() {
        if mask[row]
            && left.ok[row]
            && matches!(left.column.row_value(row), Some(Value::Bool(true)))
        {
            right_mask.set(row, true);
        }
    }
    let right = evaluate_right(&right_mask);
    merge_logical(left, right, mask, true)
}

pub(crate) fn eval_logical_or(
    left: EvalBlock,
    mask: &Mask,
    evaluate_right: impl FnOnce(&Mask) -> EvalBlock,
) -> EvalBlock {
    let mut right_mask = Mask::none(mask.len());
    for row in 0..mask.len() {
        if mask[row]
            && left.ok[row]
            && !matches!(left.column.row_value(row), Some(Value::Bool(true)))
        {
            right_mask.set(row, true);
        }
    }
    let right = evaluate_right(&right_mask);
    merge_logical(left, right, mask, false)
}

fn merge_logical(left: EvalBlock, right: EvalBlock, mask: &Mask, is_and: bool) -> EvalBlock {
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
                _ => return RowOutcome::Error(EvalError::TypeMismatch),
            };
            if (is_and && !left_value) || (!is_and && left_value) {
                return RowOutcome::Value(Value::Bool(left_value));
            }
            if !right.ok[row] {
                return RowOutcome::Failed;
            }
            match right.column.row_value(row) {
                Some(Value::Bool(value)) => RowOutcome::Value(Value::Bool(value)),
                None => RowOutcome::Null,
                _ => RowOutcome::Error(EvalError::TypeMismatch),
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
            if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        Value::Text(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Date(value) => value.to_string(),
        Value::List(values) => {
            let values = values.iter().map(stringify_value).collect::<Vec<_>>();
            format!("[{}]", values.join(", "))
        }
    }
}
