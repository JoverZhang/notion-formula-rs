// Kernel implementations opt into these helper families according to their semantics.
#![allow(dead_code)]

use crate::core::columns::{ColumnKind, KernelColumn, KernelResult, Validity};
use crate::core::errors::EvalError;
use crate::core::types::Mask;

/// Compute every physical slot for a pure, total operation, then expose only eligible rows.
pub(crate) fn eval_infallible_all_slots<I, O>(
    input: &KernelColumn<I>,
    eligible: &Mask,
    operation: impl Fn(&I::Scalar) -> O::Scalar,
) -> KernelResult<O>
where
    I: ColumnKind,
    O: ColumnKind,
{
    debug_assert_eq!(input.len(), eligible.len());
    let values = input.values().iter().map(operation).collect();
    let valid = (0..eligible.len())
        .map(|row| !eligible[row] || input.validity().is_valid(row))
        .collect();
    KernelResult {
        column: KernelColumn::from_values(values, Validity::from_valid_bits(valid)),
        ok: Mask::all(eligible.len()),
        errors: Vec::new(),
    }
}

/// Compute only active, successful, non-null rows for an operation that may fail.
pub(crate) fn eval_fallible_selected<I, O>(
    input: &KernelColumn<I>,
    eligible: &Mask,
    operation: impl Fn(&I::Scalar) -> Result<O::Scalar, EvalError>,
) -> KernelResult<O>
where
    I: ColumnKind,
    O: ColumnKind,
{
    debug_assert_eq!(input.len(), eligible.len());
    let mut values = Vec::with_capacity(input.len());
    let mut valid = Vec::with_capacity(input.len());
    let mut ok = Mask::all(eligible.len());
    let mut errors = Vec::new();
    for row in 0..eligible.len() {
        if !eligible[row] {
            values.push(O::placeholder());
            valid.push(true);
        } else if !input.validity().is_valid(row) {
            values.push(O::placeholder());
            valid.push(false);
        } else {
            match operation(&input.values()[row]) {
                Ok(value) => {
                    values.push(value);
                    valid.push(true);
                }
                Err(error) => {
                    values.push(O::placeholder());
                    valid.push(true);
                    ok.set(row, false);
                    errors.push((row, error));
                }
            }
        }
    }
    KernelResult {
        column: KernelColumn::from_values(values, Validity::from_valid_bits(valid)),
        ok,
        errors,
    }
}

/// Keep null rows eligible and let the handwritten operation define output validity.
pub(crate) fn eval_null_aware<I, O>(
    input: &KernelColumn<I>,
    eligible: &Mask,
    operation: impl Fn(Option<&I::Scalar>) -> Result<Option<O::Scalar>, EvalError>,
) -> KernelResult<O>
where
    I: ColumnKind,
    O: ColumnKind,
{
    debug_assert_eq!(input.len(), eligible.len());
    let mut values = Vec::with_capacity(input.len());
    let mut valid = Vec::with_capacity(input.len());
    let mut ok = Mask::all(eligible.len());
    let mut errors = Vec::new();
    for row in 0..eligible.len() {
        if !eligible[row] {
            values.push(O::placeholder());
            valid.push(true);
            continue;
        }
        match operation(input.value(row)) {
            Ok(Some(value)) => {
                values.push(value);
                valid.push(true);
            }
            Ok(None) => {
                values.push(O::placeholder());
                valid.push(false);
            }
            Err(error) => {
                values.push(O::placeholder());
                valid.push(true);
                ok.set(row, false);
                errors.push((row, error));
            }
        }
    }
    KernelResult {
        column: KernelColumn::from_values(values, Validity::from_valid_bits(valid)),
        ok,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use crate::core::columns::{BooleanKind, KernelColumn, NumberKind, Validity};
    use crate::core::errors::EvalError;
    use crate::core::types::Mask;

    use super::{eval_fallible_selected, eval_null_aware};

    #[test]
    fn fallible_helper_skips_null_and_inactive_placeholders() {
        let input = KernelColumn::<NumberKind>::from_values(
            vec![0.0, 0.0, 0.0],
            Validity::from_valid_bits(vec![true, false, true]),
        );
        let output = eval_fallible_selected::<_, NumberKind>(
            &input,
            &Mask::from(vec![true, true, false]),
            |value| {
                if *value == 0.0 {
                    Err(EvalError::DivideByZero)
                } else {
                    Ok(1.0 / value)
                }
            },
        );
        assert_eq!(output.errors, vec![(0, EvalError::DivideByZero)]);
        assert!(!output.ok[0]);
        assert!(output.ok[1]);
        assert!(!output.column.validity().is_valid(1));
        assert!(output.ok[2]);
        assert!(output.column.validity().is_valid(2));
    }

    #[test]
    fn null_aware_helper_can_turn_null_into_a_value() {
        let input = KernelColumn::<NumberKind>::from_values(
            vec![1.0, 0.0],
            Validity::from_valid_bits(vec![true, false]),
        );
        let output = eval_null_aware::<_, BooleanKind>(&input, &Mask::all(2), |value| {
            Ok(Some(value.is_none()))
        });
        assert_eq!(output.column.values(), &[false, true]);
        assert!(output.column.validity().is_valid(1));
    }
}
