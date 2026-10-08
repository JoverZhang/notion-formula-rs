use std::collections::HashMap;

use formula_engine::{
    Column, ColumnData, EvaluateInput, FormulaDefinition, FormulaEngine, FormulaOutput,
    FormulaSchema, NullBuffer, PropertyDefinition, RuntimeContext, ValueType,
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

#[test]
fn lowered_bindings_preserve_string_token_values_and_original_spans() {
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
    let strings = draft
        .state()
        .tokens
        .iter()
        .filter_map(|token| token.string_value().map(|value| (token.span, value)))
        .collect::<Vec<_>>();
    assert_eq!(strings.len(), 3);
    for ((span, value), id) in strings.iter().zip(["字段", "second", "字段"]) {
        assert_eq!(value, id);
        assert_eq!(
            &source[span.start as usize..span.end as usize],
            format!("\"{id}\"")
        );
    }
    assert!(
        strings
            .windows(2)
            .all(|pair| pair[0].0.end <= pair[1].0.start)
    );
}

#[test]
fn lets_string_addition_inference_matches_existing_heterogeneous_scope_cases() {
    for (source, expected_type) in [
        (
            r#"lets(a, 1, s, "v", a, a + 1, s + format(a))"#,
            ValueType::String,
        ),
        (
            r#"let(a, 7, [lets(a, "x", b, a + "y", b), a])"#,
            ValueType::List(Box::new(ValueType::Union(vec![
                ValueType::Number,
                ValueType::String,
            ]))),
        ),
    ] {
        let e = engine(source);
        let draft = e
            .create_draft(FormulaDefinition {
                id: "draft".into(),
                expression: source.into(),
            })
            .unwrap();
        assert_eq!(draft.state().output_type, expected_type, "{source}");
    }
}

#[test]
fn word_logical_operators_preserve_date_input_comparisons_and_draft_tokens() {
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
    assert_eq!(
        draft
            .state()
            .tokens
            .iter()
            .filter_map(|token| token.string_value())
            .collect::<Vec<_>>(),
        ["Deadline Date", "Snooze Deadline"]
    );
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
