use std::collections::HashMap;

use formula_engine::{
    Column, EvaluateInput, FormulaDefinition, FormulaEngine, FormulaOutput, FormulaSchema,
    PropertyDefinition, RuntimeContext, RuntimeError, Value, ValueType,
};

fn formula(id: &str, expression: &str) -> PropertyDefinition {
    PropertyDefinition::Formula(FormulaDefinition {
        id: id.into(),
        expression: expression.into(),
    })
}

fn engine(expression: &str) -> FormulaEngine {
    FormulaEngine::new(FormulaSchema {
        properties: vec![formula("result", expression)],
    })
    .unwrap()
}

fn request(rows: usize) -> EvaluateInput {
    EvaluateInput {
        row_ids: (0..rows).map(|row| format!("row-{row}").into()).collect(),
        columns: HashMap::new(),
        runtime: RuntimeContext {
            now: 0,
            time_zone: "+00:00".into(),
        },
        formula_ids: vec!["result".into()],
    }
}

fn output(engine: &FormulaEngine, input: &EvaluateInput) -> FormulaOutput {
    let mut result = engine.evaluate(input).unwrap();
    result
        .formulas
        .remove(&"result".into())
        .unwrap()
        .unwrap_or_else(|error| panic!("{error:?}"))
}

fn value(expression: &str) -> Option<Value> {
    let result = output(&engine(expression), &request(1));
    assert!(
        result.errors.is_empty(),
        "{expression}: {:?}",
        result.errors
    );
    match result.column {
        Column::Number(data) => data
            .validity
            .is_valid(0)
            .then(|| Value::Number(data.values[0])),
        Column::String(data) => data
            .validity
            .is_valid(0)
            .then(|| Value::String(data.values[0].clone())),
        Column::Boolean(data) => data
            .validity
            .is_valid(0)
            .then(|| Value::Boolean(data.values[0])),
        Column::List(data) => data
            .validity
            .is_valid(0)
            .then(|| Value::List(data.values[0].clone())),
        Column::Union(data) => data.validity.is_valid(0).then(|| data.values[0].clone()),
        _ => panic!("unexpected date"),
    }
}

#[test]
fn plain_text_styling_and_optional_number_formats_match_app_fixtures() {
    for (source, expected) in [
        (r#"style("Done")"#, "Done"),
        (r#"style("Done", "b", "green")"#, "Done"),
        (r#""Done".style("b").unstyle("b", "i")"#, "Done"),
        (r#"formatNumber(1234.5, "commas")"#, "1,234.5"),
        (r#"formatNumber(0.25, "percent")"#, "25%"),
        (r#"formatNumber(1500, "usd", 2)"#, "$1,500.00"),
        (r#"formatNumber(1500, "usd")"#, "$1,500.00"),
        (r#"formatNumber(1200000, "humanize")"#, "1.2M"),
        (r#"formatNumber(-1234, "humanize", 2)"#, "-1.23K"),
    ] {
        assert_eq!(
            value(source),
            Some(Value::String(expected.into())),
            "{source}"
        );
    }
    assert_eq!(value(r#"formatNumber(1, "commas", empty())"#), None);
    assert_eq!(value(r#"style(empty(), "b")"#), None);
}

#[test]
fn formatting_calls_keep_validation_strict_and_zero_row_batches_typed() {
    for source in ["style(1)", "unstyle(1)"] {
        let e = engine(source);
        let draft = e
            .create_draft(FormulaDefinition {
                id: "draft".into(),
                expression: source.into(),
            })
            .unwrap();
        assert!(!draft.state().diagnostics.is_empty(), "{source}");
    }
    for (source, ty) in [
        (r#"style("x")"#, ValueType::String),
        (r#"formatNumber(1, "humanize")"#, ValueType::String),
    ] {
        let result = output(&engine(source), &request(0));
        assert_eq!(result.output_type, ty, "{source}");
        assert!(result.errors.is_empty());
    }
    let result = output(&engine(r#"formatNumber(1, "bogus")"#), &request(1));
    assert!(matches!(
        result.errors[0].error,
        RuntimeError::InvalidValue { .. }
    ));
}

#[test]
fn moment_date_formats_support_literal_brackets_weeks_and_day_tokens() {
    for (pattern, expected) in [
        ("[Week] W", "Week 10"),
        (
            "[YYYY W dddd %] YYYY-MM-DD dddd",
            "YYYY W dddd % 2024-03-05 Tuesday",
        ),
        ("W WW Wo", "10 10 10th"),
        ("d dd ddd dddd e E", "2 Tu Tue Tuesday 2 2"),
        ("Do DDD DDDD DDDo", "5th 65 065 65th"),
        ("Y YY YYYY Q", "2024 24 2024 1"),
        (
            "H HH h hh k kk m mm s ss SSS A a Z ZZ",
            "10 10 10 10 10 10 30 30 5 05 125 AM am +00:00 +0000",
        ),
    ] {
        let source = format!(r#"formatDate(parseDate("2024-03-05T10:30:05.125Z"), "{pattern}")"#);
        assert_eq!(
            value(&source),
            Some(Value::String(expected.into())),
            "{pattern}"
        );
    }
    assert_eq!(
        value(r#"formatDate(parseDate("2021-01-01"), "[ISO] GGGG-WW [local] gggg-ww")"#),
        Some(Value::String("ISO 2020-53 local 2021-01".into()))
    );
    let mut input = request(1);
    input.runtime.now = 1709634600000;
    input.runtime.time_zone = "+08:00".into();
    let result = output(
        &engine(r#"formatDate(now(), "[Week] W dddd HH:mm Z")"#),
        &input,
    );
    assert!(result.errors.is_empty());
    let Column::String(data) = result.column else {
        panic!("expected string");
    };
    assert_eq!(data.values, ["Week 10 Tuesday 18:30 +08:00"]);
}
