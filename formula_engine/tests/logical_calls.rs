#[path = "support/builtin_eval.rs"]
mod builtin_eval;

use builtin_eval::{engine, formula, output, request, value};
use formula_engine::{
    Column, ColumnData, CompletionConfig, FormulaDefinition, FormulaEngine, FormulaSchema,
    NullBuffer, PropertyDefinition, RuntimeError, TextOffset, Value, ValueType,
};

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

#[test]
fn logical_calls_keep_validation_strict_and_zero_row_batches_typed() {
    for source in ["and()", "or()", "and(false, 1)", "or(true, 1)"] {
        let e = engine(source);
        let draft = e
            .create_draft(FormulaDefinition {
                id: "draft".into(),
                expression: source.into(),
            })
            .unwrap();
        assert!(!draft.state().diagnostics.is_empty(), "{source}");
    }
    let result = output(&engine("and(true)"), &request(0));
    assert_eq!(result.output_type, ValueType::Boolean);
    assert!(result.errors.is_empty());
}
