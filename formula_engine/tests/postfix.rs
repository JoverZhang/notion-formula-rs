//! Public integration checks: receiver-only calls must work in both Engine and
//! Draft without weakening ordinary argument validation or zero-argument calls.
use formula_engine::{
    Column, ColumnData, CompletionConfig, EvaluateInput, ExpressionUpdate, FormulaDefinition,
    FormulaEdit, FormulaEngine, FormulaOutput, FormulaSchema, NullBuffer, PropertyDefinition,
    PropertyId, RuntimeContext, TextOffset, ValueType,
};

fn evaluate(expression: &str, rows: usize) -> FormulaOutput {
    let engine = FormulaEngine::new(FormulaSchema {
        properties: vec![PropertyDefinition::Formula(FormulaDefinition {
            id: "result".into(),
            expression: expression.into(),
        })],
    })
    .unwrap();
    let result = engine
        .evaluate(&EvaluateInput {
            row_ids: (0..rows).map(|i| format!("row-{i}").into()).collect(),
            columns: Default::default(),
            formula_ids: vec!["result".into()],
            runtime: RuntimeContext {
                now: 1_700_000_000_000,
                time_zone: "+08:00".into(),
            },
        })
        .unwrap();
    result.formulas[&PropertyId::from("result")]
        .as_ref()
        .unwrap_or_else(|error| panic!("{expression}: {error:?}"))
        .clone()
}

#[test]
fn receiver_only_calls_match_prefix_calls_for_values_nulls_and_empty_batches() {
    for (method, prefix) in [
        (r#""hello".length()"#, r#"length("hello")"#),
        (r#""中🙂".upper()"#, r#"upper("中🙂")"#),
        ("[3, 1, 3].unique()", "unique([3, 1, 3])"),
        ("[1, 2, 3].sum()", "sum([1, 2, 3])"),
        ("[1, 2, 3].mean()", "mean([1, 2, 3])"),
        ("[5, 6].first()", "first([5, 6])"),
        ("42.format()", "format(42)"),
        ("empty().empty()", "empty(empty())"),
        ("empty().length()", "length(empty())"),
        ("[1, empty(), 2].flat()", "flat([1, empty(), 2])"),
        ("[[]].flat().length()", "length(flat([[]]))"),
    ] {
        for rows in [0, 2] {
            let actual = evaluate(method, rows);
            let expected = evaluate(prefix, rows);
            assert_eq!(actual, expected, "{method} for {rows} rows");
            assert!(actual.errors.is_empty(), "{method}");
        }
    }
    assert_eq!(
        evaluate("[1, 2, 3].sum()", 2).column,
        Column::Number(ColumnData {
            values: vec![6.0, 6.0],
            validity: NullBuffer::new_valid(2),
        })
    );
}

#[test]
fn draft_completes_a_receiver_only_call_and_renders_its_signature() {
    let engine = FormulaEngine::new(FormulaSchema { properties: vec![] }).unwrap();
    let expression = r#""中🙂".up"#;
    let mut draft = engine
        .create_draft(FormulaDefinition {
            id: "candidate".into(),
            expression: expression.into(),
        })
        .unwrap();
    let help = draft.help(TextOffset(expression.len()), CompletionConfig::default());
    let completion = help
        .completion
        .items
        .iter()
        .find(|item| item.label == ".upper()")
        .expect("single-argument functions participate in postfix completion");
    assert!(!completion.is_disabled);
    let update = draft
        .update_expression(ExpressionUpdate::Edits {
            edit: FormulaEdit {
                base_version: help.base_version,
                edits: std::iter::once(completion.primary_edit.clone().unwrap())
                    .chain(completion.additional_edits.clone())
                    .collect(),
            },
            cursor: TextOffset(expression.len()),
        })
        .unwrap();
    assert_eq!(update.state.definition.expression, r#""中🙂".upper()"#);
    assert_eq!(update.state.output_type, ValueType::String);
    assert!(update.state.diagnostics.is_empty());
    let help = draft.help(
        TextOffset(update.state.definition.expression.len() - 1),
        CompletionConfig::default(),
    );
    let signature = help.signature_help.expect("method signature");
    assert_eq!(signature.signatures.len(), 1);
    assert!(
        signature.signatures[0]
            .segments
            .iter()
            .any(|segment| matches!(
                segment,
                formula_engine::DisplaySegment::Param {
                    param_index: None,
                    ..
                }
            ))
    );
    assert!(
        !signature.signatures[0]
            .segments
            .iter()
            .any(|segment| matches!(
                segment,
                formula_engine::DisplaySegment::Param {
                    param_index: Some(_),
                    ..
                }
            ))
    );
}

#[test]
fn invalid_receivers_and_zero_parameter_functions_keep_diagnostics() {
    let engine = FormulaEngine::new(FormulaSchema { properties: vec![] }).unwrap();
    for expression in ["true.sum()", "1.now()", "1.id()", r#"true.prop("missing")"#] {
        let draft = engine
            .create_draft(FormulaDefinition {
                id: "candidate".into(),
                expression: expression.into(),
            })
            .unwrap();
        assert!(!draft.state().diagnostics.is_empty(), "{expression}");
    }
    assert_eq!(evaluate("now()", 1).output_type, ValueType::Date);
    assert_eq!(evaluate("id()", 1).output_type, ValueType::String);
    assert_eq!(evaluate("empty()", 1).output_type, ValueType::Unknown);
}
