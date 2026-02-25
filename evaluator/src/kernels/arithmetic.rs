use crate::core::errors::EvalError;
use crate::core::types::{Column, ColumnBlock, EvalBlock, Mask, Value};

use super::prepared::PreparedArgs;
use super::stringify::stringify_list;

pub(crate) fn exec_add_f64(
    prepared: PreparedArgs<'_>,
    mask: &Mask,
    errors: Vec<(usize, EvalError)>,
) -> EvalBlock {
    exec_f64_binary(prepared, mask, errors, |left, right| Ok(left + right))
}

pub(crate) fn exec_sub_f64(
    prepared: PreparedArgs<'_>,
    mask: &Mask,
    errors: Vec<(usize, EvalError)>,
) -> EvalBlock {
    exec_f64_binary(prepared, mask, errors, |left, right| Ok(left - right))
}

pub(crate) fn exec_mul_f64(
    prepared: PreparedArgs<'_>,
    mask: &Mask,
    errors: Vec<(usize, EvalError)>,
) -> EvalBlock {
    exec_f64_binary(prepared, mask, errors, |left, right| Ok(left * right))
}

pub(crate) fn exec_div_f64(
    prepared: PreparedArgs<'_>,
    mask: &Mask,
    errors: Vec<(usize, EvalError)>,
) -> EvalBlock {
    exec_f64_binary(prepared, mask, errors, |left, right| {
        if right == 0.0 {
            Err(EvalError::DivideByZero)
        } else {
            Ok(left / right)
        }
    })
}

pub(crate) fn exec_add_any(
    prepared: PreparedArgs<'_>,
    mask: &Mask,
    mut errors: Vec<(usize, EvalError)>,
) -> EvalBlock {
    let PreparedArgs::Any(args) = prepared else {
        return EvalBlock::fail_mask(mask, EvalError::TypeMismatch);
    };

    let len = mask.len();
    let mut out = vec![Value::Number(0.0); len];
    let mut nulls = vec![true; len];
    let mut ok = vec![false; len];

    for idx in 0..len {
        if !mask[idx] || !args.left_ok[idx] || !args.right_ok[idx] {
            continue;
        }
        if args.left_nulls[idx] || args.right_nulls[idx] {
            nulls[idx] = true;
            ok[idx] = true;
            continue;
        }

        let left = row_value(args.left, idx);
        let right = row_value(args.right, idx);
        match eval_add(left, right) {
            Ok(value) => {
                out[idx] = value;
                nulls[idx] = false;
                ok[idx] = true;
            }
            Err(error) => {
                errors.push((idx, error));
                nulls[idx] = true;
                ok[idx] = false;
            }
        }
    }

    EvalBlock {
        values: ColumnBlock {
            column: Column::Any(out),
            nulls,
        },
        ok,
        errors,
    }
}

pub(crate) fn eval_add(left: RowValue<'_>, right: RowValue<'_>) -> Result<Value, EvalError> {
    match (left, right) {
        (RowValue::Number(left), RowValue::Number(right)) => Ok(Value::Number(left + right)),
        (RowValue::Text(left), RowValue::Text(right)) => Ok(Value::Text(format!("{left}{right}"))),
        (RowValue::Text(left), RowValue::Number(right)) => Ok(Value::Text(format!("{left}{right}"))),
        (RowValue::Number(left), RowValue::Text(right)) => Ok(Value::Text(format!("{left}{right}"))),
        (RowValue::Text(left), RowValue::List(right)) => {
            Ok(Value::Text(format!("{left}{}", stringify_list(right))))
        }
        (RowValue::List(left), RowValue::Text(right)) => {
            Ok(Value::Text(format!("{}{right}", stringify_list(left))))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

fn exec_f64_binary<F>(
    prepared: PreparedArgs<'_>,
    mask: &Mask,
    mut errors: Vec<(usize, EvalError)>,
    op: F,
) -> EvalBlock
where
    F: Fn(f64, f64) -> Result<f64, EvalError>,
{
    let PreparedArgs::F64(args) = prepared else {
        return EvalBlock::fail_mask(mask, EvalError::TypeMismatch);
    };

    let len = mask.len();
    let mut out = vec![0.0_f64; len];
    let mut nulls = vec![true; len];
    let mut ok = vec![false; len];

    if is_all_true(mask)
        && is_all_true(args.left_ok)
        && is_all_true(args.right_ok)
        && args.left_nulls.iter().all(|is_null| !*is_null)
        && args.right_nulls.iter().all(|is_null| !*is_null)
    {
        for idx in 0..len {
            match op(args.left[idx], args.right[idx]) {
                Ok(value) => {
                    out[idx] = value;
                    nulls[idx] = false;
                    ok[idx] = true;
                }
                Err(error) => {
                    errors.push((idx, error));
                    nulls[idx] = true;
                    ok[idx] = false;
                }
            }
        }

        return EvalBlock {
            values: ColumnBlock {
                column: Column::F64(out),
                nulls,
            },
            ok,
            errors,
        };
    }

    for idx in 0..len {
        if !mask[idx] || !args.left_ok[idx] || !args.right_ok[idx] {
            continue;
        }
        if args.left_nulls[idx] || args.right_nulls[idx] {
            nulls[idx] = true;
            ok[idx] = true;
            continue;
        }

        match op(args.left[idx], args.right[idx]) {
            Ok(value) => {
                out[idx] = value;
                nulls[idx] = false;
                ok[idx] = true;
            }
            Err(error) => {
                errors.push((idx, error));
                nulls[idx] = true;
                ok[idx] = false;
            }
        }
    }

    EvalBlock {
        values: ColumnBlock {
            column: Column::F64(out),
            nulls,
        },
        ok,
        errors,
    }
}

fn is_all_true(mask: &Mask) -> bool {
    mask.iter().all(|active| *active)
}

#[derive(Clone, Copy)]
pub(crate) enum RowValue<'a> {
    Number(f64),
    Text(&'a str),
    List(&'a [Value]),
    Other,
}

fn row_value(column: &Column, idx: usize) -> RowValue<'_> {
    match column {
        Column::F64(values) => RowValue::Number(values[idx]),
        Column::Any(values) => match &values[idx] {
            Value::Number(value) => RowValue::Number(*value),
            Value::Text(value) => RowValue::Text(value.as_str()),
            Value::Bool(_) => RowValue::Other,
            Value::Date(_) => RowValue::Other,
            Value::List(values) => RowValue::List(values),
        },
    }
}
