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
            Value::Number(_) => Some(ValueType::Number),
            Value::Boolean(_) => Some(ValueType::Boolean),
            _ => None,
        };
        if let Some(expected_type) = expected_type {
            assert_eq!(draft.state().output_type, expected_type, "{source}");
        }
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
        (
            "map([1,2], index)",
            ValueType::List(Box::new(ValueType::Number)),
        ),
    ] {
        let result = output(&engine(source), &request(0));
        assert_eq!(result.output_type, ty, "{source}");
        assert!(result.errors.is_empty());
    }
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
}
