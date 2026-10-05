use std::collections::HashMap;

use formula_engine::{
    Column, ColumnData, CompletionConfig, EvaluateInput, FormulaDefinition, FormulaEngine,
    FormulaOutput, FormulaSchema, NullBuffer, PropertyDefinition, RuntimeContext, RuntimeError,
    TextOffset, Value, ValueType,
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
fn boolean_calls_share_operator_null_rules_and_short_circuit() {
    for (source, expected) in [
        ("and(true)", Some(true)),
        ("or(false)", Some(false)),
        ("and(true, false, test(\"x\", \"[\"))", Some(false)),
        ("or(false, true, test(\"x\", \"[\"))", Some(true)),
        ("and(empty(), true)", Some(false)),
        ("or(empty(), false)", Some(false)),
        ("and(true, empty())", None),
        ("or(false, empty())", None),
    ] {
        assert_eq!(value(source), expected.map(Value::Boolean), "{source}");
    }
    let engine = FormulaEngine::new(FormulaSchema {
        properties: vec![
            PropertyDefinition::Input {
                id: "flag".into(),
                ty: ValueType::Boolean,
            },
            formula("origin", "test(\"x\", \"[\")"),
            formula("result", "and(prop(\"flag\"), prop(\"origin\"))"),
        ],
    })
    .unwrap();
    let mut input = request(2);
    input.columns.insert(
        "flag".into(),
        Column::Boolean(ColumnData {
            values: vec![false, true],
            validity: NullBuffer::new_valid(2),
        }),
    );
    let result = output(&engine, &input);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].row_index, 1);
    assert_eq!(result.errors[0].origin_formula_id.0, "origin");
}

#[test]
fn lets_evaluates_sequential_heterogeneous_bindings_and_restores_shadowed_names() {
    for (source, expected) in [
        (r#"lets(a, 1, b, a + 1, a + b)"#, Value::Number(3.0)),
        (
            r#"lets(a, 1, s, "v", a, a + 1, s + format(a))"#,
            Value::String("v2".into()),
        ),
        (
            r#"let(a, 7, [lets(a, "x", b, a + "y", b), a])"#,
            Value::List(vec![
                Some(Value::String("xy".into())),
                Some(Value::Number(7.0)),
            ]),
        ),
        (r#"lets(a, empty(), empty(a))"#, Value::Boolean(true)),
    ] {
        let e = engine(source);
        let draft = e
            .create_draft(FormulaDefinition {
                id: "draft".into(),
                expression: source.into(),
            })
            .unwrap();
        assert!(
            draft.state().diagnostics.is_empty(),
            "{source}: {:?}",
            draft.state().diagnostics
        );
        let expected_type = match &expected {
            Value::Number(_) => ValueType::Number,
            Value::String(_) => ValueType::String,
            Value::Boolean(_) => ValueType::Boolean,
            _ => ValueType::List(Box::new(ValueType::Union(vec![
                ValueType::Number,
                ValueType::String,
            ]))),
        };
        assert_eq!(draft.state().output_type, expected_type, "{source}");
        drop(draft);
        assert_eq!(value(source), Some(expected), "{source}");
    }
    assert_eq!(
        value("if(false, lets(x, test(\"x\", \"[\"), x), true)"),
        Some(Value::Boolean(true))
    );
}

#[test]
fn list_callbacks_bind_zero_based_index_with_nested_and_lexical_shadowing() {
    let numbers = |values: &[f64]| {
        Some(Value::List(
            values.iter().map(|n| Some(Value::Number(*n))).collect(),
        ))
    };
    assert_eq!(value("map([10,20,30], index)"), numbers(&[0.0, 1.0, 2.0]));
    assert_eq!(
        value("let(index, 100, [map([10,20], index), index])"),
        Some(Value::List(vec![
            numbers(&[0.0, 1.0]),
            Some(Value::Number(100.0))
        ]))
    );
    assert_eq!(
        value("map([10,20], [current, index, map([3,4], current + index), current + index])"),
        Some(Value::List(vec![
            Some(Value::List(vec![
                Some(Value::Number(10.0)),
                Some(Value::Number(0.0)),
                numbers(&[3.0, 5.0]),
                Some(Value::Number(10.0))
            ])),
            Some(Value::List(vec![
                Some(Value::Number(20.0)),
                Some(Value::Number(1.0)),
                numbers(&[3.0, 5.0]),
                Some(Value::Number(21.0))
            ])),
        ]))
    );
    for (source, expected) in [
        ("filter([4,5,6], index > 0)", numbers(&[5.0, 6.0])),
        ("find([4,5,6], index == 1)", Some(Value::Number(5.0))),
        ("findIndex([4,5,6], index == 1)", Some(Value::Number(1.0))),
        ("some([4,5,6], index == 1)", Some(Value::Boolean(true))),
        ("every([4,5,6], index < 3)", Some(Value::Boolean(true))),
        ("count([4,5,6], index > 0)", Some(Value::Number(2.0))),
        ("map([empty(), 4], index)", numbers(&[0.0, 1.0])),
        ("map([], test(\"x\", \"[\"))", Some(Value::List(vec![]))),
    ] {
        assert_eq!(value(source), expected, "{source}");
    }
}

#[test]
fn optional_ifs_else_preserves_ordinary_null_and_skipped_errors() {
    for (source, expected) in [
        ("ifs(false, 1)", None),
        ("ifs(empty(), 1)", None),
        ("ifs(false, 1, true, 2)", Some(Value::Number(2.0))),
        ("ifs(false, 1, false, 2, 3)", Some(Value::Number(3.0))),
        (
            "ifs(true, true, true, test(\"x\", \"[\"))",
            Some(Value::Boolean(true)),
        ),
    ] {
        assert_eq!(value(source), expected, "{source}");
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
fn lowered_bindings_preserve_source_order_and_exact_original_reference_spans() {
    let source = r#"lets(a, prop( /* first */ "字段" ), b, [prop("second")].map(current + index), a + prop("字段"))"#;
    let e = FormulaEngine::new(FormulaSchema {
        properties: vec![
            PropertyDefinition::Input {
                id: "字段".into(),
                ty: ValueType::Number,
            },
            PropertyDefinition::Input {
                id: "second".into(),
                ty: ValueType::Number,
            },
        ],
    })
    .unwrap();
    let draft = e
        .create_draft(FormulaDefinition {
            id: "draft".into(),
            expression: source.into(),
        })
        .unwrap();
    assert!(
        draft.state().diagnostics.is_empty(),
        "{:?}",
        draft.state().diagnostics
    );
    let refs = &draft.state().property_references;
    assert_eq!(refs.len(), 3);
    for (reference, (id, call)) in refs.iter().zip([
        ("字段", r#"prop( /* first */ "字段" )"#),
        ("second", r#"prop("second")"#),
        ("字段", r#"prop("字段")"#),
    ]) {
        assert_eq!(reference.property_id.0, id);
        assert_eq!(
            &source[reference.span.start as usize..reference.span.end as usize],
            call
        );
        assert_eq!(
            &source[reference.id_span.start as usize..reference.id_span.end as usize],
            format!("\"{id}\"")
        );
    }
    assert!(
        refs.windows(2)
            .all(|pair| pair[0].span.end <= pair[1].span.start)
    );
}

#[test]
fn compatibility_calls_keep_validation_strict_and_zero_row_batches_typed() {
    for source in [
        "and()",
        "or()",
        "and(false, 1)",
        "or(true, 1)",
        "lets(1, 2, 3)",
        "lets(x, 1)",
        "ifs(true)",
        "ifs(1, 2)",
        "style(1)",
        "unstyle(1)",
        "concat([1])",
        "sum()",
    ] {
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
        ("and(true)", ValueType::Boolean),
        ("lets(x, 1, x)", ValueType::Number),
        ("ifs(false, 1)", ValueType::Number),
        (r#"style("x")"#, ValueType::String),
        (r#"formatNumber(1, "humanize")"#, ValueType::String),
        (
            "map([1,2], index)",
            ValueType::List(Box::new(ValueType::Number)),
        ),
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

#[test]
fn word_logical_operators_preserve_calls_precedence_nulls_and_original_source() {
    for (source, expected) in [
        ("true and false", Some(false)),
        ("true or test(\"x\", \"[\")", Some(true)),
        ("false and test(\"x\", \"[\")", Some(false)),
        ("false and true or true", Some(true)),
        ("true or false and false", Some(true)),
        ("not(false) and not true or true", Some(true)),
        ("and(true, not(false)) or false", Some(true)),
        ("empty() and true", Some(false)),
        ("empty() or false", Some(false)),
        ("true and empty()", None),
    ] {
        let e = engine(source);
        let draft = e
            .create_draft(FormulaDefinition {
                id: "candidate".into(),
                expression: source.into(),
            })
            .unwrap();
        assert!(
            draft.state().diagnostics.is_empty(),
            "{source}: {:?}",
            draft.state().diagnostics
        );
        assert_eq!(draft.state().definition.expression, source);
        assert_eq!(value(source), expected.map(Value::Boolean), "{source}");
    }
    let source = "true and false or not true";
    let e = engine(source);
    let draft = e
        .create_draft(FormulaDefinition {
            id: "candidate".into(),
            expression: source.into(),
        })
        .unwrap();
    assert_eq!(
        draft.format_edits().unwrap().edits[0].new_text,
        format!("{source}\n")
    );
    let incomplete = "true and ";
    let draft = e
        .create_draft(FormulaDefinition {
            id: "incomplete".into(),
            expression: incomplete.into(),
        })
        .unwrap();
    assert!(
        draft
            .help(TextOffset(incomplete.len()), CompletionConfig::default())
            .completion
            .items
            .iter()
            .any(|item| item.label == "true")
    );
    for source in ["1 and true", "false or 1"] {
        let result = output(&engine(source), &request(1));
        assert_eq!(result.errors.len(), 1, "{source}");
        assert!(
            matches!(
                result.errors[0].error,
                RuntimeError::InvalidValueType {
                    expected: ValueType::Boolean,
                    actual: ValueType::Number
                }
            ),
            "{source}: {:?}",
            result.errors
        );
    }

    let source = r#"today() >= prop("Deadline Date") and today() <= prop("Snooze Deadline")"#;
    let e = FormulaEngine::new(FormulaSchema {
        properties: vec![
            PropertyDefinition::Input {
                id: "Deadline Date".into(),
                ty: ValueType::Date,
            },
            PropertyDefinition::Input {
                id: "Snooze Deadline".into(),
                ty: ValueType::Date,
            },
            formula("result", source),
        ],
    })
    .unwrap();
    let draft = e
        .create_draft(FormulaDefinition {
            id: "candidate".into(),
            expression: source.into(),
        })
        .unwrap();
    assert!(
        draft.state().diagnostics.is_empty(),
        "{:?}",
        draft.state().diagnostics
    );
    assert_eq!(draft.state().output_type, ValueType::Boolean);
    assert_eq!(draft.state().property_references.len(), 2);
    let mut input = request(1);
    for (id, date) in [("Deadline Date", -86400000), ("Snooze Deadline", 86400000)] {
        input.columns.insert(
            id.into(),
            Column::Date(ColumnData {
                values: vec![date],
                validity: NullBuffer::new_valid(1),
            }),
        );
    }
    let result = output(&e, &input);
    assert!(result.errors.is_empty());
    let Column::Boolean(data) = result.column else {
        panic!("expected boolean");
    };
    assert_eq!(data.values, [true]);
}
