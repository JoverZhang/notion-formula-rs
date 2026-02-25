use crate::core::errors::EvalError;
use crate::core::types::{Column, ColumnBlock, EvalBlock, Mask, Value};

pub(crate) fn cast_block_to_f64(input: EvalBlock, mask: &Mask) -> EvalBlock {
    let len = input.len();
    match input.values.column {
        Column::F64(_) => input,
        Column::Any(values) => {
            let mut out = vec![0.0_f64; len];
            let mut nulls = input.values.nulls.clone();
            let mut ok = input.ok.clone();
            let mut errors = input.errors.clone();

            for idx in 0..len {
                if !mask[idx] || !ok[idx] {
                    continue;
                }
                if nulls[idx] {
                    continue;
                }
                match &values[idx] {
                    Value::Number(value) => {
                        out[idx] = *value;
                        nulls[idx] = false;
                    }
                    _ => {
                        errors.push((idx, EvalError::TypeMismatch));
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
    }
}
