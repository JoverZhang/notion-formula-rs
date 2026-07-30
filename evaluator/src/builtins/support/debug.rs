#[cfg(debug_assertions)]
use analyzer::analysis::Ty;
#[cfg(debug_assertions)]
use builtin_fn::{normalize_union, type_accepts};

use crate::builtins::BuiltinKey;
#[cfg(debug_assertions)]
use crate::core::types::Value;
use crate::core::types::{EvalBlock, Mask};
use crate::ir::{DebugArgumentContract, DebugCallContract};

use super::{EvaluatedArgument, LambdaBindings};

#[cfg(debug_assertions)]
pub(crate) fn assert_materialized_argument(
    contract: Option<&DebugArgumentContract>,
    block: &EvalBlock,
    mask: &Mask,
) {
    let Some(contract) = contract else {
        return;
    };
    assert_runtime_type_rows(
        &format!(
            "builtin argument {:?} group {:?}",
            contract.parameter, contract.repeat_group
        ),
        block,
        mask,
        &contract.expected_ty,
    );
}

#[cfg(not(debug_assertions))]
pub(crate) fn assert_materialized_argument(
    _contract: Option<&DebugArgumentContract>,
    _block: &EvalBlock,
    _mask: &Mask,
) {
}

#[cfg(debug_assertions)]
pub(crate) fn assert_lambda_bindings(
    contract: Option<&DebugArgumentContract>,
    bindings: &LambdaBindings,
    mask: &Mask,
) {
    let Some(contract) = contract else {
        return;
    };
    let Ty::Fn { params, .. } = &contract.expected_ty else {
        panic!(
            "builtin argument {:?} group {:?} expected a lambda contract, observed {}",
            contract.parameter, contract.repeat_group, contract.expected_ty
        );
    };
    assert_eq!(
        params.len(),
        bindings.as_slice().len(),
        "builtin lambda binding count for argument {:?} group {:?}",
        contract.parameter,
        contract.repeat_group
    );
    for ((_, expected_ty), (name, column)) in params.iter().zip(bindings.as_slice()) {
        let block = EvalBlock::new(column.clone(), Mask::all(mask.len()), Vec::new());
        assert_runtime_type_rows(
            &format!(
                "builtin lambda binding {name} for argument {:?} group {:?}",
                contract.parameter, contract.repeat_group
            ),
            &block,
            mask,
            expected_ty,
        );
    }
}

#[cfg(not(debug_assertions))]
pub(crate) fn assert_lambda_bindings(
    _contract: Option<&DebugArgumentContract>,
    _bindings: &LambdaBindings,
    _mask: &Mask,
) {
}

#[cfg(debug_assertions)]
pub(super) fn assert_debug_inputs(
    key: BuiltinKey,
    arguments: &[EvaluatedArgument],
    execution_mask: &Mask,
    contract: Option<&DebugCallContract>,
) {
    let Some(contract) = contract else {
        return;
    };
    for argument in arguments {
        let expected = contract.arguments.iter().find(|expected| {
            expected.parameter == argument.parameter
                && expected.repeat_group == argument.repeat_group
        });
        let Some(expected) = expected else {
            panic!(
                "builtin {} has no resolved contract for {:?} group {:?}",
                key.name(),
                argument.parameter,
                argument.repeat_group
            );
        };
        assert_runtime_type_rows(
            &format!(
                "builtin {} argument {:?} group {:?}",
                key.name(),
                argument.parameter,
                argument.repeat_group
            ),
            &argument.block,
            execution_mask,
            &expected.expected_ty,
        );
    }
}

#[cfg(not(debug_assertions))]
pub(super) fn assert_debug_inputs(
    _key: BuiltinKey,
    _arguments: &[EvaluatedArgument],
    _execution_mask: &Mask,
    _contract: Option<&DebugCallContract>,
) {
}

#[cfg(debug_assertions)]
pub(crate) fn assert_debug_output(
    key: BuiltinKey,
    block: &EvalBlock,
    execution_mask: &Mask,
    type_check_mask: &Mask,
    contract: Option<&DebugCallContract>,
) {
    assert_eq!(
        block.len(),
        execution_mask.len(),
        "builtin {} output length",
        key.name()
    );
    assert_eq!(
        block.ok.len(),
        execution_mask.len(),
        "builtin {} ok length",
        key.name()
    );
    assert_eq!(type_check_mask.len(), execution_mask.len());
    if let Some(length) = block.validity().bitmap_len() {
        assert_eq!(
            length,
            execution_mask.len(),
            "builtin {} validity length",
            key.name()
        );
    }
    for (row, _) in &block.errors {
        assert!(
            *row < execution_mask.len(),
            "builtin {} error row out of bounds",
            key.name()
        );
        assert!(
            execution_mask[*row],
            "builtin {} error row {} was not executed",
            key.name(),
            row
        );
        assert!(
            !block.ok[*row],
            "builtin {} error row remains ok",
            key.name()
        );
    }
    for row in 0..execution_mask.len() {
        let has_error = block.errors.iter().any(|(error_row, _)| *error_row == row);
        assert!(
            block.ok[row] || has_error,
            "builtin {} failed row {} has no error",
            key.name(),
            row
        );
        if !execution_mask[row] {
            assert!(
                block.ok[row],
                "builtin {} inactive row {} failed",
                key.name(),
                row
            );
            assert!(
                block.validity().is_valid(row),
                "builtin {} inactive row {} became null",
                key.name(),
                row
            );
        }
    }
    let Some(contract) = contract else {
        return;
    };
    assert_runtime_type_rows(
        &format!("builtin {} return", key.name()),
        block,
        type_check_mask,
        &contract.return_ty,
    );
}

#[cfg(not(debug_assertions))]
pub(crate) fn assert_debug_output(
    _key: BuiltinKey,
    _block: &EvalBlock,
    _execution_mask: &Mask,
    _type_check_mask: &Mask,
    _contract: Option<&DebugCallContract>,
) {
}

#[cfg(debug_assertions)]
pub(crate) fn assert_runtime_type_rows(
    context: &str,
    block: &EvalBlock,
    mask: &Mask,
    expected: &Ty,
) {
    assert_eq!(block.len(), mask.len(), "{context} mask length");
    for row in 0..block.len() {
        if !mask[row] || !block.ok[row] || !block.validity().is_valid(row) {
            continue;
        }
        let value = block.column.row_value(row).expect("valid row");
        let actual = runtime_ty(&value);
        assert!(
            type_accepts(expected, &actual),
            "{context} row {row} expected {expected}, observed {actual}"
        );
    }
}

#[cfg(debug_assertions)]
pub(crate) fn runtime_ty(value: &Value) -> Ty {
    match value {
        Value::Number(_) => Ty::Number,
        Value::Text(_) => Ty::String,
        Value::Bool(_) => Ty::Boolean,
        Value::Date(_) => Ty::Date,
        Value::List(values) if values.is_empty() => Ty::List(Box::new(Ty::Unknown)),
        Value::List(values) => Ty::List(Box::new(normalize_union(
            values.iter().map(runtime_ty).collect::<Vec<_>>(),
        ))),
    }
}
