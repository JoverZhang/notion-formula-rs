use analyzer::analysis::{Property, Ty};
use evaluator::{
    BooleanKind, BuiltinRuntimeContext, Column, EvalContext, EvalInputsBuilder, KernelColumn,
    PreparedFormula, RowBatch, RowId, Validity, Value, prepare_formula,
};

fn runtime() -> BuiltinRuntimeContext {
    BuiltinRuntimeContext::new(1_700_000_123_456, 8 * 60)
}

fn batch(len: usize) -> RowBatch {
    RowBatch::new((0..len).map(|index| RowId::from(format!("row-{index}"))), 7)
}

fn prepare(source: &str) -> PreparedFormula {
    prepare_with_properties(source, &[])
}

fn prepare_with_properties(source: &str, properties: &[(&str, Ty)]) -> PreparedFormula {
    let mut syntax = analyzer::analyze_syntax(source);
    assert!(
        syntax.diagnostics.is_empty(),
        "parse diagnostics for `{source}`: {:?}",
        syntax.diagnostics
    );
    let properties = properties
        .iter()
        .map(|(name, ty)| Property {
            name: (*name).to_string(),
            ty: ty.clone(),
            disabled_reason: None,
        })
        .collect();
    prepare_formula(&mut syntax.expr, &EvalContext::new(properties))
        .unwrap_or_else(|error| panic!("failed to prepare `{source}`: {error:?}"))
}

fn evaluate(source: &str, len: usize) -> evaluator::EvalBlock {
    let prepared = prepare(source);
    let inputs = EvalInputsBuilder::new(runtime())
        .finish(&prepared, len)
        .expect("formula has no property inputs");
    prepared
        .evaluate(batch(len), inputs)
        .expect("matching prepared input layout")
}

fn evaluate_with_columns(
    source: &str,
    properties: &[(&str, Ty)],
    columns: &[(&str, Column)],
    len: usize,
) -> evaluator::EvalBlock {
    let prepared = prepare_with_properties(source, properties);
    let mut builder = EvalInputsBuilder::new(runtime());
    for required in prepared.required_columns() {
        let (_, column) = columns
            .iter()
            .find(|(name, _)| *name == required.name)
            .unwrap_or_else(|| panic!("missing test column {}", required.name));
        builder.insert(required.slot, column.clone());
    }
    let inputs = builder
        .finish(&prepared, len)
        .expect("test columns satisfy the prepared contract");
    prepared
        .evaluate(batch(len), inputs)
        .expect("matching prepared input layout")
}

fn values(block: &evaluator::EvalBlock) -> Vec<Option<Value>> {
    (0..block.len())
        .map(|row| block.column.row_value(row))
        .collect()
}

#[test]
fn flat_recursively_flattens_the_resolved_dynamic_list() {
    let output = evaluate(r#"flat([[1], [2, ["three"]]])"#, 1);

    assert_eq!(
        values(&output),
        vec![Some(Value::List(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Text("three".to_string()),
        ]))]
    );
    assert_eq!(output.ok.as_slice(), &[true]);
    assert!(output.errors.is_empty());
}

#[test]
fn concat_evaluates_every_repeat_group_in_order() {
    let output = evaluate("concat([1], [2, 3], [4])", 1);

    assert_eq!(
        values(&output),
        vec![Some(Value::List(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(4.0),
        ]))]
    );
    assert_eq!(output.ok.as_slice(), &[true]);
    assert!(output.errors.is_empty());
}

#[test]
fn splice_combines_its_fixed_head_with_zero_or_more_items() {
    let output = evaluate("splice([1, 2, 3], 1, 1, 9, 8)", 1);

    assert_eq!(
        values(&output),
        vec![Some(Value::List(vec![
            Value::Number(1.0),
            Value::Number(9.0),
            Value::Number(8.0),
            Value::Number(3.0),
        ]))]
    );
    assert_eq!(output.ok.as_slice(), &[true]);
    assert!(output.errors.is_empty());
}

#[test]
fn ifs_selects_branches_lazily_and_isolates_row_errors() {
    let first = Column::Boolean(KernelColumn::<BooleanKind>::from_values(
        vec![true, false, false],
        Validity::AllValid,
    ));
    let second = Column::Boolean(KernelColumn::<BooleanKind>::from_values(
        vec![true, false, true],
        Validity::AllValid,
    ));
    let output = evaluate_with_columns(
        r#"ifs(prop("First"), 10, prop("Second"), divide(1, 0), 30)"#,
        &[("First", Ty::Boolean), ("Second", Ty::Boolean)],
        &[("First", first), ("Second", second)],
        3,
    );

    assert_eq!(output.column.row_value(0), Some(Value::Number(10.0)));
    assert_eq!(output.column.row_value(1), Some(Value::Number(30.0)));
    assert_eq!(output.ok.as_slice(), &[true, true, false]);
    assert_eq!(output.errors, vec![(2, evaluator::EvalError::DivideByZero)]);
    assert!(output.validity().is_valid(2));
}

#[test]
fn ifs_stops_evaluating_conditions_after_the_first_match() {
    let first = Column::Boolean(KernelColumn::<BooleanKind>::from_values(
        vec![true, false],
        Validity::AllValid,
    ));
    let output = evaluate_with_columns(
        r#"ifs(prop("First"), 10, divide(1, 0) > 0, 20, 30)"#,
        &[("First", Ty::Boolean)],
        &[("First", first)],
        2,
    );

    assert_eq!(output.column.row_value(0), Some(Value::Number(10.0)));
    assert_eq!(output.ok.as_slice(), &[true, false]);
    assert_eq!(output.errors, vec![(1, evaluator::EvalError::DivideByZero)]);
}

#[test]
fn runtime_snapshot_and_row_ids_feed_system_builtins() {
    let now = evaluate("now()", 2);
    assert_eq!(
        values(&now),
        vec![
            Some(Value::Date(1_700_000_123_456)),
            Some(Value::Date(1_700_000_123_456)),
        ]
    );

    let today = evaluate("today()", 1);
    assert_eq!(values(&today), vec![Some(Value::Date(1_699_977_600_000))]);

    let ids = evaluate("id()", 2);
    assert_eq!(
        values(&ids),
        vec![
            Some(Value::Text("row-0".to_string())),
            Some(Value::Text("row-1".to_string())),
        ]
    );
}

// Each case below failed against the initial PR3 implementation during its semantic audit.
#[test]
fn observed_value_regressions_remain_fixed() {
    let cases = [
        ("empty(0)", Value::Bool(true)),
        ("empty(false)", Value::Bool(true)),
        ("sign(0)", Value::Number(0.0)),
        ("sign(-0)", Value::Number(0.0)),
        (r#"substring("abc", 2, 1)"#, Value::Text(String::new())),
        ("timestamp(fromTimestamp(42))", Value::Number(0.0)),
        (
            r#"formatDate(parseDate("2024-05-06"), "MMMM D, Y")"#,
            Value::Text("May 6, 2024".to_string()),
        ),
        (
            r#"format(parseDate("2024-05-06"))"#,
            Value::Text("May 6, 2024 00:00".to_string()),
        ),
        (
            r#"dateBetween(parseDate("2024-01-01"), parseDate("2023-01-01"), "years")"#,
            Value::Number(1.0),
        ),
        (
            r#"formatNumber(1234.5, "usd", 2)"#,
            Value::Text("$1,234.50".to_string()),
        ),
    ];

    for (source, expected) in cases {
        let output = evaluate(source, 1);
        assert_eq!(values(&output), vec![Some(expected)], "{source}");
        assert_eq!(output.ok.as_slice(), &[true], "{source}");
        assert!(output.errors.is_empty(), "{source}");
    }
}
