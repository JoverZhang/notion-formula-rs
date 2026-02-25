use crate::core::types::{Column, ColumnBlock, EvalBlock, Mask, Value};

pub(crate) fn literal_f64(value: f64, len: usize, mask: &Mask) -> EvalBlock {
    let mut rows = vec![0.0; len];
    let mut nulls = vec![true; len];
    let mut ok = vec![false; len];
    for (idx, active) in mask.iter().copied().enumerate() {
        if active {
            rows[idx] = value;
            nulls[idx] = false;
            ok[idx] = true;
        }
    }

    EvalBlock {
        values: ColumnBlock {
            column: Column::F64(rows),
            nulls,
        },
        ok,
        errors: Vec::new(),
    }
}

pub(crate) fn literal_any(value: Value, len: usize, mask: &Mask) -> EvalBlock {
    let mut rows = vec![Value::Number(0.0); len];
    let mut nulls = vec![true; len];
    let mut ok = vec![false; len];
    for (idx, active) in mask.iter().copied().enumerate() {
        if active {
            rows[idx] = value.clone();
            nulls[idx] = false;
            ok[idx] = true;
        }
    }

    EvalBlock {
        values: ColumnBlock {
            column: Column::Any(rows),
            nulls,
        },
        ok,
        errors: Vec::new(),
    }
}
