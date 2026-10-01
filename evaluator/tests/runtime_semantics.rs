use analyzer::analysis::{Property, Ty};
use evaluator::{
    AnyKind, BooleanKind, BuiltinRuntimeContext, Column, DateKind, EvalBlock, EvalContext,
    EvalError, EvalInputsBuilder, KernelColumn, Mask, NumberKind, PreparedFormula, RowBatch, RowId,
    Validity, Value, prepare_formula,
};

fn prepare(source: &str, properties: &[(&str, Ty)]) -> PreparedFormula {
    let mut syntax = analyzer::analyze_syntax(source);
    assert!(
        syntax.diagnostics.is_empty(),
        "{source}: {:?}",
        syntax.diagnostics
    );
    let context = EvalContext::new(
        properties
            .iter()
            .map(|(name, ty)| Property {
                name: (*name).to_string(),
                ty: ty.clone(),
                disabled_reason: None,
            })
            .collect(),
    );
    prepare_formula(&mut syntax.expr, &context)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
}

fn batch(len: usize) -> RowBatch {
    RowBatch::new((0..len).map(|row| RowId::from(format!("row-{row}"))), 0)
}

fn evaluate(source: &str, properties: Vec<(&str, Ty, Column)>) -> EvalBlock {
    let types = properties
        .iter()
        .map(|(name, ty, _)| (*name, ty.clone()))
        .collect::<Vec<_>>();
    let prepared = prepare(source, &types);
    let len = properties.first().map_or(1, |(_, _, column)| column.len());
    let mut builder = EvalInputsBuilder::new(BuiltinRuntimeContext::new(0, 0));
    for required in prepared.required_columns() {
        let (_, _, column) = properties
            .iter()
            .find(|(name, _, _)| *name == required.name)
            .unwrap();
        builder.insert(required.slot, column.clone());
    }
    prepared
        .evaluate(batch(len), builder.finish(&prepared, len).unwrap())
        .unwrap()
}

fn number(values: Vec<f64>) -> Column {
    Column::Number(KernelColumn::<NumberKind>::from_values(
        values,
        Validity::AllValid,
    ))
}

fn any(values: Vec<Value>) -> Column {
    Column::Any(KernelColumn::<AnyKind>::from_values(
        values,
        Validity::AllValid,
    ))
}

fn n(value: f64) -> Option<Value> {
    Some(Value::Number(value))
}
fn list(values: Vec<Option<Value>>) -> Value {
    Value::List(values)
}

fn assert_value(source: &str, expected: Option<Value>) {
    let result = evaluate(source, vec![]);
    assert!(result.errors.is_empty(), "{source}: {:?}", result.errors);
    assert!(result.ok[0]);
    assert_eq!(result.column.row_value(0), expected, "{source}");
}

fn scalar_number(source: &str) -> f64 {
    let result = evaluate(source, vec![]);
    assert!(result.errors.is_empty(), "{source}: {:?}", result.errors);
    let Some(Value::Number(value)) = result.column.row_value(0) else {
        panic!("{source}: {result:?}");
    };
    value
}

#[test]
fn ordinary_null_is_preserved_in_nested_lists_and_list_functions() {
    for (source, expected) in [
        (r#"empty()"#, None),
        (r#"empty(empty())"#, Some(Value::Bool(true))),
        (
            r#"[1, empty(), [empty(), 2]]"#,
            Some(list(vec![n(1.0), None, Some(list(vec![None, n(2.0)]))])),
        ),
        (
            r#"map([1, empty(), 2], current)"#,
            Some(list(vec![n(1.0), None, n(2.0)])),
        ),
        (
            r#"map([1, empty(), 2], empty())"#,
            Some(list(vec![None, None, None])),
        ),
        (
            r#"map([1, empty(), 2], if(empty(current), 3, current))"#,
            Some(list(vec![n(1.0), n(3.0), n(2.0)])),
        ),
        (
            r#"flat([[1, empty()], [2], [[3]]])"#,
            Some(list(vec![n(1.0), None, n(2.0), Some(list(vec![n(3.0)]))])),
        ),
        (r#"flat([[[]]])"#, Some(list(vec![Some(list(vec![]))]))),
        (r#"sort([2, empty(), 1])"#, Some(list(vec![n(1.0), n(2.0)]))),
        (r#"sort([empty(), empty()])"#, Some(list(vec![]))),
        (
            r#"join([1, empty(), 2], ",")"#,
            Some(Value::Text("1,,2".to_string())),
        ),
        (
            r#"join([empty(), empty()], ",")"#,
            Some(Value::Text(",".to_string())),
        ),
        (r#"first([empty(), 1])"#, None),
        (r#"find([empty(), 1], empty(current))"#, None),
    ] {
        assert_value(source, expected);
    }
}

#[test]
fn null_list_arguments_return_ordinary_null() {
    for source in [
        r#"flat(empty())"#,
        r#"sort(empty())"#,
        r#"map(empty(), current)"#,
        r#"join(empty(), ",")"#,
    ] {
        assert_value(source, None);
    }
}

#[test]
fn aggregates_ignore_nulls_and_expand_only_one_level() {
    for (source, expected) in [
        (r#"sum([2, empty(), 4])"#, Some(Value::Number(6.0))),
        (r#"sum(2, empty(), 4)"#, Some(Value::Number(6.0))),
        (r#"mean([2, empty(), 4])"#, Some(Value::Number(3.0))),
        (r#"min([empty(), 4], 2)"#, Some(Value::Number(2.0))),
        (r#"max([2, empty()], 4)"#, Some(Value::Number(4.0))),
        (r#"median([empty(), 2], 4)"#, Some(Value::Number(3.0))),
        (r#"sum([])"#, Some(Value::Number(0.0))),
        (r#"sum(empty(), [empty()])"#, Some(Value::Number(0.0))),
        (r#"min([])"#, None),
        ("max([empty()])", None),
        (r#"median(empty())"#, None),
        ("mean([])", None),
    ] {
        assert_value(source, expected);
    }
    assert!(!scalar_number(r#"sum([])"#).is_sign_negative());
    let output = evaluate(
        r#"sum(prop("X"))"#,
        vec![(
            "X",
            Ty::Unknown,
            any(vec![list(vec![Some(list(vec![n(1.0)]))])]),
        )],
    );
    assert_eq!(
        output.errors,
        vec![(
            0,
            EvalError::InvalidValueType {
                expected: Ty::Number,
                actual: Ty::List(Box::new(Ty::Number)),
            }
        )]
    );
}

#[test]
fn ieee_numbers_survive_operators_math_and_aggregates() {
    for source in [
        r#"0 / 0"#,
        r#"1 % 0"#,
        r#"sqrt(-1)"#,
        r#"ln(-1)"#,
        r#"pow(1, 1/0)"#,
        r#"1 ^ (1/0)"#,
        r#"mean(1/0, -1/0)"#,
        r#"median(0/0, 1)"#,
        r#"min(1, 0/0)"#,
        r#"max(0/0, 1)"#,
        r#"sum(1, 0/0)"#,
    ] {
        assert!(scalar_number(source).is_nan(), "{source}");
    }
    for source in [
        r#"1 / 0"#,
        r#"divide(1, 0)"#,
        r#"exp(1000)"#,
        r#"sqrt(1/0)"#,
        r#"sum(1/0, 1)"#,
    ] {
        assert_eq!(scalar_number(source), f64::INFINITY, "{source}");
    }
    for source in [r#"-1 / 0"#, r#"ln(0)"#, r#"log10(0)"#, r#"log2(0)"#] {
        assert_eq!(scalar_number(source), f64::NEG_INFINITY, "{source}");
    }
    for source in [
        r#"(0/0) == (0/0)"#,
        r#"(0/0) < 1"#,
        r#"(0/0) <= 1"#,
        r#"(0/0) > 1"#,
        r#"(0/0) >= 1"#,
    ] {
        assert_value(source, Some(Value::Bool(false)));
    }
    for source in [r#"(0/0) != (0/0)"#, r#"0 == -0"#] {
        assert_value(source, Some(Value::Bool(true)));
    }
    for source in [
        r#"sign(-0)"#,
        r#"sqrt(-0)"#,
        r#"round(-0.5)"#,
        r#"ceil(-0.5)"#,
        r#"min(0, -0)"#,
        r#"median(-1, -0, 0)"#,
    ] {
        let value = scalar_number(source);
        assert_eq!(value, 0.0, "{source}");
        assert!(value.is_sign_negative(), "{source}");
    }
    for source in [r#"sum(-0)"#, r#"max(-0, 0)"#] {
        let value = scalar_number(source);
        assert_eq!(value, 0.0);
        assert!(!value.is_sign_negative(), "{source}");
    }
    assert_eq!(scalar_number(r#"round(-1.5)"#), -1.0);
    let overflow = evaluate(
        r#"prop("X") * prop("X")"#,
        vec![("X", Ty::Number, number(vec![1e308]))],
    );
    assert_eq!(
        overflow.column.row_value(0),
        Some(Value::Number(f64::INFINITY))
    );
    assert!(overflow.errors.is_empty());
}

#[test]
fn aggregates_validate_all_values_even_after_nan() {
    let value = list(vec![n(f64::NAN), Some(Value::Text("bad".to_string()))]);
    let result = evaluate(
        r#"sum(prop("X"))"#,
        vec![("X", Ty::Unknown, any(vec![value]))],
    );
    assert_eq!(
        result.errors,
        vec![(
            0,
            EvalError::InvalidValueType {
                expected: Ty::Number,
                actual: Ty::String
            }
        )]
    );
}

#[test]
fn repeat_clamps_and_rounds_counts_and_retains_constraint_payloads() {
    assert_value(r#"repeat("ab", -1.9)"#, Some(Value::Text(String::new())));
    assert_value(
        r#"repeat("ab", 1.1)"#,
        Some(Value::Text("abab".to_string())),
    );
    let result = evaluate(r#"repeat("ab", 10001)"#, vec![]);
    assert_eq!(
        result.column.row_value(0),
        Some(Value::Text("ab".repeat(10_000)))
    );
    for source in [r#"repeat("", 1/0)"#, r#"repeat("x", 1/0)"#] {
        let result = evaluate(source, vec![]);
        assert_eq!(
            result.errors,
            vec![(
                0,
                EvalError::InvalidValue {
                    actual: Value::Number(f64::INFINITY),
                    constraint: "repeat count must be finite".to_string()
                }
            )]
        );
        assert!(!result.ok[0]);
    }
    assert_value(r#"repeat(empty(), 1/0)"#, None);
    assert_value(r#"repeat("x", empty())"#, None);
    let nan = evaluate(r#"repeat("x", 0/0)"#, vec![]);
    assert!(
        matches!(&nan.errors[0].1, EvalError::InvalidValue { actual: Value::Number(value), constraint } if value.is_nan() && constraint == "repeat count must be finite")
    );
}

#[test]
fn unknown_and_union_values_fail_with_expected_and_actual_types() {
    for (source, value, expected, actual) in [
        (
            r#"abs(prop("X"))"#,
            Value::Text("x".to_string()),
            Ty::Number,
            Ty::String,
        ),
        (
            r#"prop("X") - 1"#,
            Value::Bool(true),
            Ty::Number,
            Ty::Boolean,
        ),
        (r#"!prop("X")"#, Value::Number(1.0), Ty::Boolean, Ty::Number),
        (
            r#"prop("X") ? 1 : 2"#,
            Value::Number(1.0),
            Ty::Boolean,
            Ty::Number,
        ),
        (
            r#"prop("X") || test("x", "[")"#,
            Value::Number(1.0),
            Ty::Boolean,
            Ty::Number,
        ),
        (
            r#"padStart(prop("X"), 2, "x")"#,
            Value::Bool(true),
            Ty::Union(vec![Ty::Number, Ty::String]),
            Ty::Boolean,
        ),
        (
            r#"map(prop("X"), current)"#,
            Value::Bool(true),
            Ty::List(Box::new(Ty::Unknown)),
            Ty::Boolean,
        ),
        (
            r#"sum(prop("X"))"#,
            Value::Bool(true),
            Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))]),
            Ty::Boolean,
        ),
    ] {
        let output = evaluate(source, vec![("X", Ty::Unknown, any(vec![value]))]);
        assert_eq!(
            output.errors,
            vec![(0, EvalError::InvalidValueType { expected, actual })],
            "{source}"
        );
    }
    let union = Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Unknown))]);
    let output = evaluate(
        r#"sum(prop("X"))"#,
        vec![(
            "X",
            union,
            any(vec![
                Value::Number(2.0),
                list(vec![Some(Value::Bool(true))]),
            ]),
        )],
    );
    assert_eq!(output.column.row_value(0), Some(Value::Number(2.0)));
    assert_eq!(
        output.errors,
        vec![(
            1,
            EvalError::InvalidValueType {
                expected: Ty::Number,
                actual: Ty::Boolean
            }
        )]
    );
}

#[test]
fn repeat_checks_nonnull_dynamic_types_before_null_propagation() {
    let output = evaluate(
        r#"repeat(empty(), prop("X"))"#,
        vec![("X", Ty::Unknown, any(vec![Value::Bool(true)]))],
    );
    assert_eq!(
        output.errors,
        vec![(
            0,
            EvalError::InvalidValueType {
                expected: Ty::Number,
                actual: Ty::Boolean
            }
        )]
    );
}

#[test]
fn regex_and_date_errors_keep_the_original_text_and_compile_detail() {
    let result = evaluate(r#"test("text", "[")"#, vec![]);
    assert!(
        matches!(&result.errors[0].1, EvalError::InvalidRegex { pattern, detail } if pattern == "[" && detail.contains("unclosed"))
    );
    let result = evaluate(r#"parseDate("not-a-date")"#, vec![]);
    assert_eq!(
        result.errors,
        vec![(
            0,
            EvalError::InvalidDateText {
                text: "not-a-date".to_string()
            }
        )]
    );
}

#[test]
fn date_inputs_are_allowed_until_a_date_operation_uses_them() {
    let date = || {
        Column::Date(KernelColumn::<DateKind>::from_values(
            vec![i64::MAX],
            Validity::AllValid,
        ))
    };
    let passthrough = evaluate(r#"prop("D")"#, vec![("D", Ty::Date, date())]);
    assert_eq!(passthrough.column.row_value(0), Some(Value::Date(i64::MAX)));
    assert!(passthrough.errors.is_empty());
    for source in [
        r#"year(prop("D"))"#,
        r#"timestamp(prop("D"))"#,
        r#"dateAdd(prop("D"), 0, "days")"#,
        r#"dateBetween(prop("D"), now(), "days")"#,
        r#"formatDate(prop("D"), "YYYY")"#,
        r#"format(prop("D"))"#,
    ] {
        let result = evaluate(source, vec![("D", Ty::Date, date())]);
        assert_eq!(
            result.errors,
            vec![(0, EvalError::DateOutOfRange)],
            "{source}"
        );
    }
    for source in [
        r#"parseDate("0000-01-01")"#,
        r#"parseDate("+10000-01-01")"#,
        r#"dateAdd(parseDate("9999-12-31"), 1, "days")"#,
        r#"dateSubtract(parseDate("0001-01-01"), 1, "days")"#,
        r#"fromTimestamp(253402300800000)"#,
    ] {
        assert_eq!(
            evaluate(source, vec![]).errors,
            vec![(0, EvalError::DateOutOfRange)],
            "{source}"
        );
    }
}

#[test]
fn date_operations_validate_utc_and_offset_adjusted_local_boundaries() {
    let epochs = [-62_135_596_800_000, 253_402_300_799_999];
    for (offset, failed_row) in [(-480, 0), (480, 1)] {
        let prepared = prepare(r#"year(prop("D"))"#, &[("D", Ty::Date)]);
        let column = Column::Date(KernelColumn::<DateKind>::from_values(
            epochs.to_vec(),
            Validity::AllValid,
        ));
        let inputs = EvalInputsBuilder::new(BuiltinRuntimeContext::new(0, offset))
            .with_column(prepared.required_columns()[0].slot, column)
            .finish(&prepared, 2)
            .unwrap();
        let result = prepared.evaluate(batch(2), inputs).unwrap();
        assert_eq!(result.errors, vec![(failed_row, EvalError::DateOutOfRange)]);
    }
    let prepared = prepare(r#"parseDate("0001-01-01")"#, &[]);
    let inputs = EvalInputsBuilder::new(BuiltinRuntimeContext::new(0, 480))
        .finish(&prepared, 1)
        .unwrap();
    assert_eq!(
        prepared.evaluate(batch(1), inputs).unwrap().errors,
        vec![(0, EvalError::DateOutOfRange)]
    );
}

fn upstream_error(occurrence: u64) -> EvalError {
    EvalError::Originated {
        origin_formula_id: "upstream".to_string(),
        occurrence,
        error: Box::new(EvalError::InvalidDateText {
            text: "bad-date".to_string(),
        }),
    }
}

#[test]
fn upstream_failures_propagate_only_through_executed_controlled_and_lambda_branches() {
    for source in [
        r#"if(prop("C"), prop("U"), 7)"#,
        r#"ifs(prop("C"), prop("U"), 7)"#,
        r#"prop("C") ? prop("U") : 7"#,
        r#"if(prop("C"), let(x, prop("U"), x + 1), 7)"#,
        r#"map([false, prop("C")], if(current, prop("U"), 7))"#,
        r#"prop("C") && prop("U") > 0"#,
        r#"!prop("C") || prop("U") > 0"#,
    ] {
        let prepared = prepare(source, &[("C", Ty::Boolean), ("U", Ty::Number)]);
        let mut builder = EvalInputsBuilder::new(BuiltinRuntimeContext::new(0, 0));
        for required in prepared.required_columns() {
            match required.name.as_str() {
                "C" => {
                    builder.insert(
                        required.slot,
                        Column::Boolean(KernelColumn::<BooleanKind>::from_values(
                            vec![false, true],
                            Validity::AllValid,
                        )),
                    );
                }
                "U" => {
                    builder.insert_block(
                        required.slot,
                        EvalBlock::new(
                            number(vec![0.0, 0.0]),
                            Mask::none(2),
                            vec![(0, upstream_error(10)), (1, upstream_error(11))],
                        ),
                    );
                }
                _ => unreachable!(),
            }
        }
        let result = prepared
            .evaluate(batch(2), builder.finish(&prepared, 2).unwrap())
            .unwrap();
        assert_eq!(result.ok.as_slice(), &[true, false], "{source}");
        assert_eq!(result.errors, vec![(1, upstream_error(11))], "{source}");
    }
}

#[test]
fn input_block_errors_follow_outer_masks_and_repeated_reads_keep_identity() {
    let prepared = prepare(r#"prop("U") + prop("U")"#, &[("U", Ty::Number)]);
    let block = EvalBlock::new(
        number(vec![0.0, 0.0]),
        Mask::none(2),
        vec![(0, upstream_error(10)), (1, upstream_error(11))],
    );
    let inputs = EvalInputsBuilder::new(BuiltinRuntimeContext::new(0, 0))
        .with_block(prepared.required_columns()[0].slot, block)
        .finish(&prepared, 2)
        .unwrap();
    let result = prepared
        .evaluate_with_mask(batch(2), inputs, Mask::from(vec![false, true]))
        .unwrap();
    assert_eq!(result.ok.as_slice(), &[true, false]);
    assert!(result.validity().is_valid(0));
    assert_eq!(
        result.errors,
        vec![(1, upstream_error(11)), (1, upstream_error(11))]
    );
    let local = evaluate(r#"repeat("x", 1/0) + repeat("x", 1/0)"#, vec![]);
    assert_eq!(local.errors.len(), 2);
    assert_eq!(local.errors[0], local.errors[1]);
}

#[test]
fn prepared_formula_exposes_the_resolved_output_type() {
    assert_eq!(prepare(r#"empty()"#, &[]).output_type(), &Ty::Unknown);
    assert_eq!(prepare(r#"empty(1)"#, &[]).output_type(), &Ty::Boolean);
    assert_eq!(
        prepare(r#"flat([[[]]])"#, &[]).output_type(),
        &Ty::List(Box::new(Ty::List(Box::new(Ty::Unknown))))
    );
}

#[test]
fn unbound_variables_are_rejected_before_runtime_but_lexical_scopes_remain_executable() {
    let mut syntax = analyzer::analyze_syntax("missing");
    assert!(
        matches!(prepare_formula(&mut syntax.expr, &EvalContext::new(vec![])), Err(evaluator::PrepareError::UnboundVariable(name)) if name == "missing")
    );
    assert_value("let(x, 1, let(x, 2, x) + x)", Some(Value::Number(3.0)));
    assert_value(
        "let(x, 1, first(map([2], current + x)))",
        Some(Value::Number(3.0)),
    );
}
