use analyzer::analysis::{Property, Ty};

use crate::builtins::{
    BuiltinEvaluationMode, BuiltinKey, EvaluatedArgument, PreparedValueArguments,
    assert_debug_output,
};
use crate::core::columns::{
    BooleanKind, Column, KernelColumn, NumberKind, SharedBitmap, TextKind, Validity,
};
use crate::core::context::{BuiltinRuntimeContext, EvalContext};
use crate::core::errors::{EvalError, InputContractError};
use crate::core::inputs::EvalInputsBuilder;
use crate::core::types::{Mask, RowBatch, RowId, Value};
use crate::{PreparedFormula, prepare_formula};
use builtin_fn::ParamRef;

fn runtime() -> BuiltinRuntimeContext {
    BuiltinRuntimeContext::new(1_700_000_123_456, 8 * 60)
}

fn batch(len: usize) -> RowBatch {
    RowBatch::new((0..len).map(|index| RowId::from(format!("row-{index}"))), 7)
}

fn context(properties: &[(&str, Ty)]) -> EvalContext {
    EvalContext::new(
        properties
            .iter()
            .map(|(name, ty)| Property {
                name: (*name).to_string(),
                ty: ty.clone(),
                disabled_reason: None,
            })
            .collect(),
    )
}

fn prepare(source: &str, properties: &[(&str, Ty)]) -> PreparedFormula {
    let mut syntax = analyzer::analyze_syntax(source);
    assert!(
        syntax.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        syntax.diagnostics
    );
    prepare_formula(&mut syntax.expr, &context(properties))
        .unwrap_or_else(|error| panic!("failed to prepare `{source}`: {error:?}"))
}

fn finish_empty(prepared: &PreparedFormula, len: usize) -> crate::EvalInputs {
    EvalInputsBuilder::new(runtime())
        .finish(prepared, len)
        .unwrap()
}

fn number_column(values: Vec<f64>, validity: Validity) -> Column {
    Column::Number(KernelColumn::<NumberKind>::from_values(values, validity))
}

fn bool_column(values: Vec<bool>) -> Column {
    Column::Boolean(KernelColumn::<BooleanKind>::from_values(
        values,
        Validity::AllValid,
    ))
}

fn text_column(values: Vec<&str>) -> Column {
    Column::Text(KernelColumn::<TextKind>::from_values(
        values.into_iter().map(str::to_string).collect(),
        Validity::AllValid,
    ))
}

fn values(block: &crate::EvalBlock) -> Vec<Option<Value>> {
    (0..block.len())
        .map(|row| block.column.row_value(row))
        .collect()
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
fn required_columns_are_complete_deduplicated_and_first_seen() {
    let prepared = prepare(
        r#"if(true, prop("B"), prop("A")) + prop("B")"#,
        &[("A", Ty::Number), ("B", Ty::Number)],
    );
    let names = prepared
        .required_columns()
        .iter()
        .map(|column| column.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["B", "A"]);
    assert_eq!(prepared.required_columns()[0].slot.index(), 0);
    assert_eq!(prepared.required_columns()[1].slot.index(), 1);
}

#[test]
fn input_contract_reports_all_five_error_classes() {
    let prepared = prepare(r#"prop("A")"#, &[("A", Ty::Number)]);
    let slot = prepared.required_columns()[0].slot;

    assert_eq!(
        EvalInputsBuilder::new(runtime())
            .finish(&prepared, 2)
            .unwrap_err(),
        InputContractError::MissingColumn {
            slot,
            name: "A".to_string(),
        }
    );

    let mut duplicate = EvalInputsBuilder::new(runtime());
    duplicate.insert(slot, number_column(vec![1.0, 2.0], Validity::AllValid));
    duplicate.insert(slot, number_column(vec![3.0, 4.0], Validity::AllValid));
    assert_eq!(
        duplicate.finish(&prepared, 2).unwrap_err(),
        InputContractError::DuplicateColumn { slot }
    );

    let wrong_kind = EvalInputsBuilder::new(runtime())
        .with_column(slot, text_column(vec!["a", "b"]))
        .finish(&prepared, 2);
    assert_eq!(
        wrong_kind.unwrap_err(),
        InputContractError::WrongKind {
            slot,
            expected: crate::AbiKind::Number,
            actual: crate::AbiKind::Text,
        }
    );

    let wrong_length = EvalInputsBuilder::new(runtime())
        .with_column(slot, number_column(vec![1.0], Validity::AllValid))
        .finish(&prepared, 2);
    assert_eq!(
        wrong_length.unwrap_err(),
        InputContractError::WrongLength {
            slot,
            expected: 2,
            actual: 1,
        }
    );

    let other = prepare(r#"prop("A")"#, &[("A", Ty::Number)]);
    let other_slot = other.required_columns()[0].slot;
    let wrong_layout = EvalInputsBuilder::new(runtime())
        .with_column(
            other_slot,
            number_column(vec![1.0, 2.0], Validity::AllValid),
        )
        .finish(&prepared, 2);
    assert_eq!(
        wrong_layout.unwrap_err(),
        InputContractError::WrongInputLayout
    );

    let inputs = EvalInputsBuilder::new(runtime())
        .with_column(slot, number_column(vec![1.0, 2.0], Validity::AllValid))
        .finish(&prepared, 2)
        .unwrap();
    assert_eq!(
        other.evaluate(batch(2), inputs),
        Err(InputContractError::WrongInputLayout)
    );
}

#[test]
fn execution_mask_row_ok_and_null_validity_are_independent() {
    let prepared = prepare(r#"prop("A")"#, &[("A", Ty::Number)]);
    let slot = prepared.required_columns()[0].slot;
    let column = number_column(
        vec![0.0, 2.0, 3.0],
        Validity::Bitmap(SharedBitmap::new(vec![false, true, false])),
    );
    let storage_probe = column.clone();
    let inputs = EvalInputsBuilder::new(runtime())
        .with_column(slot, column)
        .finish(&prepared, 3)
        .unwrap();
    let output = prepared
        .evaluate_with_mask(batch(3), inputs, Mask::from(vec![true, true, false]))
        .unwrap();

    assert_eq!(output.ok.as_slice(), &[true, true, true]);
    assert!(!output.validity().is_valid(0));
    assert!(output.validity().is_valid(1));
    assert!(output.validity().is_valid(2));
    assert!(output.column.shares_storage_with(&storage_probe));
    assert!(output.errors.is_empty());
}

#[test]
fn row_error_does_not_become_null_or_execution_mask() {
    let prepared = prepare(
        r#"divide(prop("A"), prop("B"))"#,
        &[("A", Ty::Number), ("B", Ty::Number)],
    );
    let mut builder = EvalInputsBuilder::new(runtime());
    builder.insert(
        prepared.required_columns()[0].slot,
        number_column(
            vec![1.0, 0.0, 3.0],
            Validity::Bitmap(SharedBitmap::new(vec![true, false, true])),
        ),
    );
    builder.insert(
        prepared.required_columns()[1].slot,
        number_column(vec![0.0, 2.0, 0.0], Validity::AllValid),
    );
    let output = prepared
        .evaluate_with_mask(
            batch(3),
            builder.finish(&prepared, 3).unwrap(),
            Mask::from(vec![true, true, false]),
        )
        .unwrap();

    assert_eq!(output.ok.as_slice(), &[false, true, true]);
    assert!(output.validity().is_valid(0));
    assert!(!output.validity().is_valid(1));
    assert!(output.validity().is_valid(2));
    assert_eq!(output.errors, vec![(0, EvalError::DivideByZero)]);
}

#[test]
fn shared_column_fan_out_clones_only_the_handle() {
    let original = number_column(vec![1.0, 2.0, 3.0], Validity::AllValid);
    let clone = original.clone();
    assert!(original.shares_storage_with(&clone));
    match (original, clone) {
        (Column::Number(original), Column::Number(clone)) => {
            assert_eq!(original.storage_strong_count(), 2);
            assert!(original.shares_storage_with(&clone));
        }
        _ => unreachable!(),
    }
}

#[test]
fn typed_dispatch_moves_a_matching_column_handle_without_copying_rows() {
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
        unreachable!()
    };
    assert!(taken.shares_storage_with(&storage_probe));
}

#[test]
fn unique_column_storage_can_be_recovered_without_copying_rows() {
    let column = KernelColumn::<NumberKind>::from_values(vec![1.0, 2.0, 3.0], Validity::AllValid);
    let (mut storage, validity) = column.try_into_unique().expect("unique storage");
    storage[1] = 9.0;
    let column = KernelColumn::<NumberKind>::from_owned(storage, validity);
    assert_eq!(column.values(), &[1.0, 9.0, 3.0]);
}

#[test]
fn fixed_runtime_snapshot_and_row_ids_feed_system_builtins() {
    let now = prepare("now()", &[]);
    let output = now.evaluate(batch(2), finish_empty(&now, 2)).unwrap();
    assert_eq!(
        values(&output),
        vec![
            Some(Value::Date(1_700_000_123_456)),
            Some(Value::Date(1_700_000_123_456)),
        ]
    );

    let today = prepare("today()", &[]);
    let output = today.evaluate(batch(2), finish_empty(&today, 2)).unwrap();
    assert_eq!(
        values(&output),
        vec![
            Some(Value::Date(1_699_977_600_000)),
            Some(Value::Date(1_699_977_600_000)),
        ]
    );

    let id = prepare("id()", &[]);
    let output = id.evaluate(batch(2), finish_empty(&id, 2)).unwrap();
    assert_eq!(
        values(&output),
        vec![
            Some(Value::Text("row-0".to_string())),
            Some(Value::Text("row-1".to_string())),
        ]
    );
}

#[test]
fn extreme_runtime_dates_become_row_errors_instead_of_panics() {
    let today = prepare("today()", &[]);
    let inputs = EvalInputsBuilder::new(BuiltinRuntimeContext::new(i64::MAX, 1))
        .finish(&today, 1)
        .unwrap();
    let output = today.evaluate(batch(1), inputs).unwrap();
    assert_eq!(output.ok.as_slice(), &[false]);
    assert_eq!(output.errors, vec![(0, EvalError::InvalidDate)]);

    for source in [
        r#"dateSubtract(fromTimestamp(0), -1e30, "milliseconds")"#,
        r#"dateBetween(fromTimestamp(1e30), fromTimestamp(-1e30), "milliseconds")"#,
    ] {
        let prepared = prepare(source, &[]);
        let output = prepared
            .evaluate(batch(1), finish_empty(&prepared, 1))
            .unwrap();
        assert_eq!(output.ok.as_slice(), &[false], "{source}");
        assert_eq!(output.errors, vec![(0, EvalError::InvalidDate)], "{source}");
    }
}

#[test]
fn representative_value_builtins_cover_fixed_repeat_and_head_repeat() {
    let flat = prepare(r#"flat([[1], [2, ["three"]]])"#, &[]);
    let output = flat.evaluate(batch(1), finish_empty(&flat, 1)).unwrap();
    assert_eq!(
        values(&output),
        vec![Some(Value::List(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Text("three".to_string()),
        ]))]
    );

    let concat = prepare("concat([1], [2, 3])", &[]);
    let output = concat.evaluate(batch(1), finish_empty(&concat, 1)).unwrap();
    assert_eq!(
        values(&output),
        vec![Some(Value::List(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]))]
    );

    let splice = prepare("splice([1, 2, 3], 1, 1, 9, 8)", &[]);
    let output = splice.evaluate(batch(1), finish_empty(&splice, 1)).unwrap();
    assert_eq!(
        values(&output),
        vec![Some(Value::List(vec![
            Value::Number(1.0),
            Value::Number(9.0),
            Value::Number(8.0),
            Value::Number(3.0),
        ]))]
    );
}

#[test]
fn planner_inserts_an_explicit_cast_between_generic_and_concrete_abis() {
    let prepared = prepare("abs(if(true, -1, -2))", &[]);
    let output = prepared
        .evaluate(batch(1), finish_empty(&prepared, 1))
        .unwrap();
    assert_eq!(output.column.abi_kind(), crate::AbiKind::Number);
    assert_eq!(values(&output), vec![Some(Value::Number(1.0))]);
}

#[test]
fn ifs_uses_branch_masks_and_isolates_unselected_errors() {
    let prepared = prepare(
        r#"ifs(prop("First"), 10, prop("Second"), divide(1, 0), 30)"#,
        &[("First", Ty::Boolean), ("Second", Ty::Boolean)],
    );
    let mut builder = EvalInputsBuilder::new(runtime());
    builder.insert(
        prepared.required_columns()[0].slot,
        bool_column(vec![true, false, false]),
    );
    builder.insert(
        prepared.required_columns()[1].slot,
        bool_column(vec![true, false, true]),
    );
    let output = prepared
        .evaluate(batch(3), builder.finish(&prepared, 3).unwrap())
        .unwrap();

    assert_eq!(
        values(&output),
        vec![
            Some(Value::Number(10.0)),
            Some(Value::Number(30.0)),
            Some(Value::Number(0.0)),
        ]
    );
    assert_eq!(output.ok.as_slice(), &[true, true, false]);
    assert_eq!(output.errors, vec![(2, EvalError::DivideByZero)]);
}

#[test]
fn ifs_does_not_evaluate_later_conditions_after_a_match() {
    let prepared = prepare(
        r#"ifs(prop("First"), 10, divide(1, 0) > 0, 20, 30)"#,
        &[("First", Ty::Boolean)],
    );
    let inputs = EvalInputsBuilder::new(runtime())
        .with_column(
            prepared.required_columns()[0].slot,
            bool_column(vec![true, false]),
        )
        .finish(&prepared, 2)
        .unwrap();
    let output = prepared.evaluate(batch(2), inputs).unwrap();
    assert_eq!(output.ok.as_slice(), &[true, false]);
    assert_eq!(values(&output)[0], Some(Value::Number(10.0)));
    assert_eq!(output.errors, vec![(1, EvalError::DivideByZero)]);
}

#[test]
fn conditional_operators_do_not_evaluate_unselected_errors() {
    let cases = [
        ("if(false, divide(1, 0), 1)", Value::Number(1.0)),
        ("false && divide(1, 0) > 0", Value::Bool(false)),
        ("true || divide(1, 0) > 0", Value::Bool(true)),
    ];
    for (source, expected) in cases {
        let prepared = prepare(source, &[]);
        let output = prepared
            .evaluate(batch(1), finish_empty(&prepared, 1))
            .unwrap();
        assert_eq!(values(&output), vec![Some(expected)], "{source}");
        assert_eq!(output.ok.as_slice(), &[true], "{source}");
        assert!(output.errors.is_empty(), "{source}");
    }
}

#[cfg(debug_assertions)]
#[test]
fn shared_debug_output_contract_rejects_inactive_errors_and_nulls() {
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
