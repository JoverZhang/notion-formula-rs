use crate::core::errors::EvalError;
use crate::core::types::{Column, EvalBlock, Mask, NullMap};

pub(crate) struct F64PreparedArgs<'a> {
    pub left: &'a [f64],
    pub right: &'a [f64],
    pub left_nulls: &'a NullMap,
    pub right_nulls: &'a NullMap,
    pub left_ok: &'a Mask,
    pub right_ok: &'a Mask,
}

pub(crate) struct AnyPreparedArgs<'a> {
    pub left: &'a Column,
    pub right: &'a Column,
    pub left_nulls: &'a NullMap,
    pub right_nulls: &'a NullMap,
    pub left_ok: &'a Mask,
    pub right_ok: &'a Mask,
}

pub(crate) enum PreparedArgs<'a> {
    F64(F64PreparedArgs<'a>),
    Any(AnyPreparedArgs<'a>),
}

pub(crate) fn prepare_f64_args<'a>(
    left: &'a EvalBlock,
    right: &'a EvalBlock,
    mask: &Mask,
) -> Result<PreparedArgs<'a>, EvalBlock> {
    match (&left.values.column, &right.values.column) {
        (Column::F64(left_values), Column::F64(right_values)) => {
            Ok(PreparedArgs::F64(F64PreparedArgs {
                left: left_values,
                right: right_values,
                left_nulls: &left.values.nulls,
                right_nulls: &right.values.nulls,
                left_ok: &left.ok,
                right_ok: &right.ok,
            }))
        }
        _ => Err(type_mismatch_prepare_block(mask, left, right)),
    }
}

pub(crate) fn prepare_any_args<'a>(
    left: &'a EvalBlock,
    right: &'a EvalBlock,
    _mask: &Mask,
) -> Result<PreparedArgs<'a>, EvalBlock> {
    Ok(PreparedArgs::Any(AnyPreparedArgs {
        left: &left.values.column,
        right: &right.values.column,
        left_nulls: &left.values.nulls,
        right_nulls: &right.values.nulls,
        left_ok: &left.ok,
        right_ok: &right.ok,
    }))
}

fn type_mismatch_prepare_block(mask: &Mask, left: &EvalBlock, right: &EvalBlock) -> EvalBlock {
    let len = mask.len();
    let mut errors = Vec::new();
    for idx in 0..len {
        if !mask[idx] || !left.ok[idx] || !right.ok[idx] {
            continue;
        }
        if left.values.nulls[idx] || right.values.nulls[idx] {
            continue;
        }
        errors.push((idx, EvalError::TypeMismatch));
    }

    EvalBlock {
        values: crate::core::types::ColumnBlock {
            column: Column::F64(vec![0.0; len]),
            nulls: vec![true; len],
        },
        ok: vec![false; len],
        errors,
    }
}
