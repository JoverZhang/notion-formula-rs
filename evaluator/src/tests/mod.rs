use analyzer::analysis::Ty;
use builtin_fn::ParamRef;

use crate::builtins::{
    BuiltinEvaluationMode, BuiltinKey, EvaluatedArgument, PreparedValueArguments,
};
#[cfg(debug_assertions)]
use crate::builtins::{LambdaBindings, assert_debug_output, assert_lambda_bindings};
use crate::core::columns::{AnyKind, Column, KernelColumn, NumberKind, Validity};
#[cfg(debug_assertions)]
use crate::core::columns::{SharedBitmap, TextKind};
#[cfg(debug_assertions)]
use crate::core::errors::EvalError;
use crate::core::types::Mask;
use crate::core::types::Value;
#[cfg(debug_assertions)]
use crate::ir::DebugArgumentContract;

fn number_column(values: Vec<f64>, validity: Validity) -> Column {
    Column::Number(KernelColumn::<NumberKind>::from_values(values, validity))
}

#[test]
fn generated_catalog_has_one_obligation_per_supported_builtin() {
    let supported = builtin_fn::builtins_functions();
    assert_eq!(BuiltinKey::ALL.len(), supported.len());
    assert_eq!(BuiltinKey::ALL.len(), 83);
    for (key, signature) in BuiltinKey::ALL.iter().zip(supported) {
        assert_eq!(key.name(), signature.name);
        let expected_mode = if signature
            .display_params()
            .iter()
            .any(|param| matches!(param.ty, Ty::Fn { .. } | Ty::Ident(_)))
        {
            BuiltinEvaluationMode::Controlled
        } else {
            BuiltinEvaluationMode::Value
        };
        assert_eq!(key.evaluation_mode(), expected_mode);
    }
}

#[test]
fn typed_dispatch_preparation_moves_a_column_handle_without_copying_rows() {
    let column = number_column(vec![1.0, 2.0], Validity::AllValid);
    let storage_probe = column.clone();
    let mask = Mask::all(2);
    let mut prepared = PreparedValueArguments::new(
        vec![EvaluatedArgument {
            parameter: ParamRef::Head(0),
            repeat_group: None,
            block: crate::EvalBlock::new(column, Mask::all(2), Vec::new()),
        }],
        &mask,
        BuiltinKey::Abs,
        None,
    );
    let taken = prepared
        .take_value::<NumberKind>(ParamRef::Head(0), None)
        .expect("matching physical ABI");
    let Column::Number(storage_probe) = storage_probe else {
        panic!("expected number storage probe");
    };
    assert!(taken.shares_storage_with(&storage_probe));
}

#[cfg(debug_assertions)]
#[test]
fn typed_dispatch_panics_on_a_wrong_physical_abi() {
    let mask = Mask::all(1);
    let result = std::panic::catch_unwind(|| {
        let column = Column::Any(KernelColumn::<AnyKind>::from_values(
            vec![Value::Number(1.0)],
            Validity::AllValid,
        ));
        let mut prepared = PreparedValueArguments::new(
            vec![EvaluatedArgument {
                parameter: ParamRef::Head(0),
                repeat_group: None,
                block: crate::EvalBlock::new(column, Mask::all(1), Vec::new()),
            }],
            &mask,
            BuiltinKey::Abs,
            None,
        );
        let _ = prepared.take_value::<NumberKind>(ParamRef::Head(0), None);
    });

    assert!(result.is_err());
}

#[cfg(not(debug_assertions))]
#[test]
fn typed_dispatch_returns_an_error_on_a_wrong_physical_abi_in_release() {
    let column = Column::Any(KernelColumn::<AnyKind>::from_values(
        vec![Value::Number(1.0)],
        Validity::AllValid,
    ));
    let mask = Mask::all(1);
    let mut prepared = PreparedValueArguments::new(
        vec![EvaluatedArgument {
            parameter: ParamRef::Head(0),
            repeat_group: None,
            block: crate::EvalBlock::new(column, Mask::all(1), Vec::new()),
        }],
        &mask,
        BuiltinKey::Abs,
        None,
    );

    assert!(
        prepared
            .take_value::<NumberKind>(ParamRef::Head(0), None)
            .is_err()
    );
}

#[cfg(debug_assertions)]
#[test]
fn lambda_binding_contract_rejects_an_incompatible_runtime_type() {
    let contract = DebugArgumentContract {
        parameter: ParamRef::Head(0),
        repeat_group: None,
        expected_ty: Ty::Fn {
            params: vec![(builtin_fn::LambdaParam::Current, Ty::Number)],
            ret: Box::new(Ty::Number),
        },
    };
    let bindings = LambdaBindings::new(vec![(
        "current".to_string(),
        Column::Text(KernelColumn::<TextKind>::from_values(
            vec!["wrong".to_string()],
            Validity::AllValid,
        )),
    )]);

    assert!(
        std::panic::catch_unwind(|| {
            assert_lambda_bindings(Some(&contract), &bindings, &Mask::all(1));
        })
        .is_err()
    );
}

#[cfg(debug_assertions)]
#[test]
fn shared_debug_contract_rejects_inactive_errors_and_nulls() {
    let error_outside_mask = crate::EvalBlock::new(
        number_column(vec![1.0, 0.0], Validity::AllValid),
        Mask::from(vec![true, false]),
        vec![(1, EvalError::InvalidArgument)],
    );
    assert!(
        std::panic::catch_unwind(|| {
            assert_debug_output(
                BuiltinKey::Flat,
                &error_outside_mask,
                &Mask::from(vec![true, false]),
                &Mask::from(vec![true, false]),
                None,
            );
        })
        .is_err()
    );

    let inactive_null = crate::EvalBlock::new(
        number_column(
            vec![1.0, 0.0],
            Validity::Bitmap(SharedBitmap::new(vec![true, false])),
        ),
        Mask::all(2),
        Vec::new(),
    );
    assert!(
        std::panic::catch_unwind(|| {
            assert_debug_output(
                BuiltinKey::Ifs,
                &inactive_null,
                &Mask::from(vec![true, false]),
                &Mask::from(vec![true, false]),
                None,
            );
        })
        .is_err()
    );
}
