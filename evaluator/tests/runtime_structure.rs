use analyzer::analysis::{Property, Ty};
use evaluator::{
    AbiKind, BuiltinRuntimeContext, Column, EvalContext, EvalInputsBuilder, InputContractError,
    KernelColumn, Mask, NumberKind, PreparedFormula, RowBatch, RowId, SharedBitmap, TextKind,
    Validity, Value, prepare_formula,
};

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

fn finish_empty(prepared: &PreparedFormula, len: usize) -> evaluator::EvalInputs {
    EvalInputsBuilder::new(runtime())
        .finish(prepared, len)
        .expect("empty input contract")
}

fn number_column(values: Vec<f64>, validity: Validity) -> Column {
    Column::Number(KernelColumn::<NumberKind>::from_values(values, validity))
}

fn text_column(values: Vec<&str>) -> Column {
    Column::Text(KernelColumn::<TextKind>::from_values(
        values.into_iter().map(str::to_string).collect(),
        Validity::AllValid,
    ))
}

#[test]
fn required_columns_are_complete_deduplicated_and_first_seen() {
    let prepared = prepare(
        r#"if(true, prop("B"), prop("A")) + prop("B")"#,
        &[("A", Ty::Number), ("B", Ty::Number)],
    );
    assert_eq!(
        prepared
            .required_columns()
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        ["B", "A"]
    );
    assert_eq!(prepared.required_columns()[0].slot.index(), 0);
    assert_eq!(prepared.required_columns()[1].slot.index(), 1);
}

#[test]
fn input_builder_reports_all_five_contract_error_classes() {
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

    assert_eq!(
        EvalInputsBuilder::new(runtime())
            .with_column(slot, text_column(vec!["a", "b"]))
            .finish(&prepared, 2)
            .unwrap_err(),
        InputContractError::WrongKind {
            slot,
            expected: AbiKind::Number,
            actual: AbiKind::Text,
        }
    );

    assert_eq!(
        EvalInputsBuilder::new(runtime())
            .with_column(slot, number_column(vec![1.0], Validity::AllValid))
            .finish(&prepared, 2)
            .unwrap_err(),
        InputContractError::WrongLength {
            slot,
            expected: 2,
            actual: 1,
        }
    );

    let other = prepare(r#"prop("A")"#, &[("A", Ty::Number)]);
    let other_slot = other.required_columns()[0].slot;
    assert_eq!(
        EvalInputsBuilder::new(runtime())
            .with_column(
                other_slot,
                number_column(vec![1.0, 2.0], Validity::AllValid),
            )
            .finish(&prepared, 2)
            .unwrap_err(),
        InputContractError::WrongInputLayout
    );

    let inputs = EvalInputsBuilder::new(runtime())
        .with_column(slot, number_column(vec![1.0, 2.0], Validity::AllValid))
        .finish(&prepared, 2)
        .expect("matching inputs");
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
        .expect("matching inputs");
    let output = prepared
        .evaluate_with_mask(batch(3), inputs, Mask::from(vec![true, true, false]))
        .expect("valid input layout");

    assert_eq!(output.ok.as_slice(), &[true, true, true]);
    assert!(!output.validity().is_valid(0));
    assert!(output.validity().is_valid(1));
    assert!(output.validity().is_valid(2));
    assert!(output.column.shares_storage_with(&storage_probe));
    assert!(output.errors.is_empty());
}

#[test]
fn row_error_does_not_become_null_or_execution_state() {
    let prepared = prepare(
        r#"prop("A") / prop("B")"#,
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
            builder.finish(&prepared, 3).expect("matching inputs"),
            Mask::from(vec![true, true, false]),
        )
        .expect("valid input layout");

    assert_eq!(output.ok.as_slice(), &[false, true, true]);
    assert!(output.validity().is_valid(0));
    assert!(!output.validity().is_valid(1));
    assert!(output.validity().is_valid(2));
    assert_eq!(output.errors, vec![(0, evaluator::EvalError::DivideByZero)]);
}

#[test]
fn shared_column_fan_out_clones_only_the_handle() {
    let original = KernelColumn::<NumberKind>::from_values(vec![1.0, 2.0, 3.0], Validity::AllValid);
    let clone = original.clone();
    assert_eq!(original.storage_strong_count(), 2);
    assert!(original.shares_storage_with(&clone));
}

#[test]
fn unique_column_storage_can_be_recovered_for_in_place_work() {
    let column = KernelColumn::<NumberKind>::from_values(vec![1.0, 2.0, 3.0], Validity::AllValid);
    let (mut storage, validity) = column.try_into_unique().expect("unique storage");
    storage[1] = 9.0;
    let column = KernelColumn::<NumberKind>::from_owned(storage, validity);
    assert_eq!(column.values(), &[1.0, 9.0, 3.0]);
}

#[test]
fn non_builtin_literals_and_operators_remain_executable() {
    let prepared = prepare("(1 + 2) * 3 - 4 / 2", &[]);
    let output = prepared
        .evaluate(batch(2), finish_empty(&prepared, 2))
        .expect("valid input layout");
    assert_eq!(output.ok.as_slice(), &[true, true]);
    assert!(output.errors.is_empty());
    assert_eq!(
        (0..output.len())
            .map(|row| output.column.row_value(row))
            .collect::<Vec<_>>(),
        [Some(Value::Number(7.0)), Some(Value::Number(7.0))]
    );
}
