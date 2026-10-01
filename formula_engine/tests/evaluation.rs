use std::fmt::Write;

use formula_engine::{
    Column, ColumnData, ColumnKind, EvaluateInput, EvaluateInputError, EvaluateResult,
    FormulaDefinition, FormulaEngine, FormulaEvaluationError, FormulaOutput, FormulaSchema,
    FormulaStatus, NullBuffer, PropertyDefinition, PropertyId, PropertyState, RuntimeContext,
    RuntimeError, Value, ValueType,
};

fn formula(id: &str, expression: &str) -> PropertyDefinition {
    PropertyDefinition::Formula(FormulaDefinition {
        id: id.into(),
        expression: expression.into(),
    })
}

fn input(id: &str, ty: ValueType) -> PropertyDefinition {
    PropertyDefinition::Input { id: id.into(), ty }
}

fn engine(definitions: Vec<PropertyDefinition>) -> FormulaEngine {
    FormulaEngine::new(FormulaSchema {
        properties: definitions,
    })
    .unwrap()
}

fn request(rows: usize, formula_ids: &[&str], columns: &[(&str, Column)]) -> EvaluateInput {
    EvaluateInput {
        row_ids: (0..rows).map(|row| format!("row-{row}").into()).collect(),
        columns: columns
            .iter()
            .map(|(id, column)| ((*id).into(), column.clone()))
            .collect(),
        runtime: RuntimeContext {
            now: 1_700_000_123_456,
            time_zone: "+08:00".into(),
        },
        formula_ids: formula_ids.iter().map(|id| (*id).into()).collect(),
    }
}

fn numbers(values: &[f64]) -> Column {
    Column::Number(ColumnData {
        values: values.to_vec(),
        validity: NullBuffer::new_valid(values.len()),
    })
}

fn strings(values: &[&str]) -> Column {
    Column::String(ColumnData {
        values: values.iter().map(|value| (*value).into()).collect(),
        validity: NullBuffer::new_valid(values.len()),
    })
}

fn booleans(values: &[bool]) -> Column {
    Column::Boolean(ColumnData {
        values: values.to_vec(),
        validity: NullBuffer::new_valid(values.len()),
    })
}

fn dynamic(values: &[Value]) -> Column {
    Column::Union(ColumnData {
        values: values.to_vec(),
        validity: NullBuffer::new_valid(values.len()),
    })
}

fn output<'a>(result: &'a EvaluateResult, id: &str) -> &'a FormulaOutput {
    result.formulas[&PropertyId::from(id)].as_ref().unwrap()
}

fn row_value(column: &Column, row: usize) -> Option<Value> {
    macro_rules! value {
        ($data:expr, $variant:ident) => {
            $data
                .validity
                .is_valid(row)
                .then(|| Value::$variant($data.values[row].clone()))
        };
    }
    match column {
        Column::Number(data) => value!(data, Number),
        Column::String(data) => value!(data, String),
        Column::Boolean(data) => value!(data, Boolean),
        Column::Date(data) => value!(data, Date),
        Column::List(data) => value!(data, List),
        Column::Union(data) => data
            .validity
            .is_valid(row)
            .then(|| data.values[row].clone()),
    }
}

#[test]
fn shared_dependencies_multiple_requests_and_zero_rows() {
    let engine = engine(vec![
        input("x", ValueType::Number),
        formula("shared", "prop(\"x\") + 1"),
        formula("left", "prop(\"shared\") * 2"),
        formula("right", "prop(\"shared\") + 3"),
        formula("diamond", "prop(\"left\") + prop(\"right\")"),
        formula("unrelated", "prop(\"missing\")"),
    ]);
    let ids = ["diamond", "right", "shared"];
    let result = engine
        .evaluate(&request(3, &ids, &[("x", numbers(&[0.0, 10.0, -2.0]))]))
        .unwrap();
    assert_eq!(result.formulas.len(), ids.len());
    assert_eq!(
        output(&result, "diamond").column,
        numbers(&[6.0, 36.0, 0.0])
    );
    assert_eq!(output(&result, "right").column, numbers(&[4.0, 14.0, 2.0]));
    assert_eq!(
        output(&result, "shared").column,
        numbers(&[1.0, 11.0, -1.0])
    );
    let empty = engine
        .evaluate(&request(0, &ids, &[("x", numbers(&[]))]))
        .unwrap();
    for id in ids {
        assert_eq!(output(&empty, id).output_type, ValueType::Number);
        assert_eq!(output(&empty, id).column, numbers(&[]));
        assert!(output(&empty, id).errors.is_empty());
    }
}

#[test]
fn mixed_and_all_not_ready_requests_remain_outer_successes() {
    let engine = engine(vec![
        formula("ready", "1"),
        formula("bad", "prop(\"missing\")"),
    ]);
    let mixed = engine
        .evaluate(&request(2, &["bad", "ready"], &[]))
        .unwrap();
    assert_eq!(
        mixed.formulas[&"bad".into()],
        Err(FormulaEvaluationError::NotReady)
    );
    assert_eq!(output(&mixed, "ready").column, numbers(&[1.0, 1.0]));
    let all = engine.evaluate(&request(0, &["bad"], &[])).unwrap();
    assert_eq!(
        all.formulas[&"bad".into()],
        Err(FormulaEvaluationError::NotReady)
    );
}

#[test]
fn dependency_errors_follow_executed_branches_and_keep_origins() {
    let engine = engine(vec![
        input("skip", ValueType::Boolean),
        input("pattern", ValueType::String),
        formula("source", "test(\"x\", prop(\"pattern\"))"),
        formula("conditional", "if(prop(\"skip\"), true, prop(\"source\"))"),
        formula("logical", "prop(\"skip\") || prop(\"source\")"),
        formula("repeat_read", "equal(prop(\"source\"), prop(\"source\"))"),
        formula("left", "prop(\"source\")"),
        formula("right", "prop(\"source\")"),
        formula("diamond", "equal(prop(\"left\"), prop(\"right\"))"),
        formula(
            "two_failures",
            "equal(test(\"x\", \"[\"), test(\"x\", \"[\"))",
        ),
    ]);
    let ids = [
        "source",
        "conditional",
        "logical",
        "repeat_read",
        "diamond",
        "two_failures",
    ];
    let input = request(
        3,
        &ids,
        &[
            ("skip", booleans(&[true, false, false])),
            ("pattern", strings(&["[", "[", "x"])),
        ],
    );
    let result = engine.evaluate(&input).unwrap();
    assert_eq!(
        row_value(&output(&result, "conditional").column, 0),
        Some(Value::Boolean(true))
    );
    assert_eq!(
        row_value(&output(&result, "logical").column, 0),
        Some(Value::Boolean(true))
    );
    for id in ["conditional", "logical"] {
        let output = output(&result, id);
        assert_eq!(output.errors.len(), 1);
        assert_eq!(output.errors[0].row_index, 1);
        assert_eq!(output.errors[0].origin_formula_id, "source".into());
        assert_eq!(row_value(&output.column, 1), None);
        assert_eq!(row_value(&output.column, 2), Some(Value::Boolean(true)));
    }
    for id in ["source", "repeat_read", "diamond"] {
        let output = output(&result, id);
        assert_eq!(
            output.errors.len(),
            2,
            "{id}: repeated reads must preserve one original failure per row"
        );
        assert!(
            output
                .errors
                .iter()
                .all(|error| error.origin_formula_id == "source".into())
        );
    }
    let two = output(&result, "two_failures");
    assert_eq!(
        two.errors.len(),
        6,
        "distinct identical failures must remain distinct"
    );
    assert!(
        two.errors
            .iter()
            .all(|error| error.origin_formula_id == "two_failures".into())
    );
    assert!(two.errors.iter().all(|error| matches!(&error.error,
        RuntimeError::InvalidRegex { pattern, detail } if pattern == "[" && !detail.is_empty()
    )));
    assert_eq!(
        snapshot(&result),
        snapshot(&engine.evaluate(&input).unwrap())
    );
}

#[test]
fn null_is_distinct_from_failures_and_list_positions_are_retained() {
    let engine = engine(vec![
        formula("null", "empty()"),
        formula("mapped", "map([1, empty(), 3], current)"),
        formula("joined", "join([\"a\", empty(), \"b\"], \",\")"),
        formula("flat", "flat([[1, empty()], [2], empty()])"),
        formula("one_level", "flat([[[1]]])"),
        formula("sorted", "sort([2, empty(), 1])"),
        formula("sum", "sum([2, empty(), 4])"),
        formula("mean", "mean([2, empty(), 4])"),
        formula("empty_sum", "sum([])"),
        formula("empty_mean", "mean([])"),
        formula("failure", "parseDate(\"not-a-date\")"),
    ]);
    let ids = [
        "null",
        "mapped",
        "joined",
        "flat",
        "one_level",
        "sorted",
        "sum",
        "mean",
        "empty_sum",
        "empty_mean",
        "failure",
    ];
    let result = engine.evaluate(&request(1, &ids, &[])).unwrap();
    assert_eq!(output(&result, "null").output_type, ValueType::Unknown);
    assert!(matches!(output(&result, "null").column, Column::Union(_)));
    assert_eq!(row_value(&output(&result, "null").column, 0), None);
    assert!(output(&result, "null").errors.is_empty());
    assert_eq!(
        row_value(&output(&result, "mapped").column, 0),
        Some(Value::List(vec![
            Some(Value::Number(1.0)),
            None,
            Some(Value::Number(3.0))
        ]))
    );
    assert_eq!(
        row_value(&output(&result, "joined").column, 0),
        Some(Value::String("a,,b".into()))
    );
    assert_eq!(
        row_value(&output(&result, "flat").column, 0),
        Some(Value::List(vec![
            Some(Value::Number(1.0)),
            None,
            Some(Value::Number(2.0)),
            None
        ]))
    );
    assert_eq!(
        output(&result, "one_level").output_type,
        ValueType::List(Box::new(ValueType::List(Box::new(ValueType::Number))))
    );
    assert_eq!(
        row_value(&output(&result, "one_level").column, 0),
        Some(Value::List(vec![Some(Value::List(vec![Some(
            Value::Number(1.0)
        )]))]))
    );
    assert_eq!(
        row_value(&output(&result, "sorted").column, 0),
        Some(Value::List(vec![
            Some(Value::Number(1.0)),
            Some(Value::Number(2.0))
        ]))
    );
    assert_eq!(output(&result, "sum").column, numbers(&[6.0]));
    assert_eq!(output(&result, "mean").column, numbers(&[3.0]));
    assert_eq!(output(&result, "empty_sum").column, numbers(&[0.0]));
    assert_eq!(row_value(&output(&result, "empty_mean").column, 0), None);
    assert!(output(&result, "empty_mean").errors.is_empty());
    let failed = output(&result, "failure");
    assert_eq!(row_value(&failed.column, 0), None);
    assert!(
        matches!(&failed.errors[0].error, RuntimeError::InvalidDateText { text } if text == "not-a-date")
    );
}

#[test]
fn number_rules_and_repeat_constraints_are_exposed_through_engine() {
    let engine = engine(vec![
        input("n", ValueType::Number),
        formula("infinity", "1 / 0"),
        formula("nan", "0 / 0"),
        formula("nan_equal", "equal(0 / 0, 0 / 0)"),
        formula("nan_not_equal", "unequal(0 / 0, 0 / 0)"),
        formula("nan_less", "(0 / 0) < 1"),
        formula("negative_zero", "-0"),
        formula("reciprocal", "1 / prop(\"n\")"),
        formula("repeat", "repeat(\"ab\", prop(\"n\"))"),
    ]);
    let ids = [
        "infinity",
        "nan",
        "nan_equal",
        "nan_not_equal",
        "nan_less",
        "negative_zero",
        "reciprocal",
        "repeat",
    ];
    let result = engine
        .evaluate(&request(
            5,
            &ids,
            &[("n", numbers(&[-0.0, -1.9, 1.1, f64::INFINITY, f64::NAN]))],
        ))
        .unwrap();
    let Some(Value::Number(infinity)) = row_value(&output(&result, "infinity").column, 0) else {
        panic!()
    };
    assert_eq!(infinity, f64::INFINITY);
    let Some(Value::Number(nan)) = row_value(&output(&result, "nan").column, 0) else {
        panic!()
    };
    assert!(nan.is_nan());
    assert_eq!(
        row_value(&output(&result, "nan_equal").column, 0),
        Some(Value::Boolean(false))
    );
    assert_eq!(
        row_value(&output(&result, "nan_not_equal").column, 0),
        Some(Value::Boolean(true))
    );
    assert_eq!(
        row_value(&output(&result, "nan_less").column, 0),
        Some(Value::Boolean(false))
    );
    let Some(Value::Number(zero)) = row_value(&output(&result, "negative_zero").column, 0) else {
        panic!()
    };
    assert!(zero == 0.0 && zero.is_sign_negative());
    assert_eq!(
        row_value(&output(&result, "reciprocal").column, 0),
        Some(Value::Number(f64::NEG_INFINITY))
    );
    let repeat = output(&result, "repeat");
    assert_eq!(
        row_value(&repeat.column, 0),
        Some(Value::String(String::new()))
    );
    assert_eq!(
        row_value(&repeat.column, 1),
        Some(Value::String(String::new()))
    );
    assert_eq!(
        row_value(&repeat.column, 2),
        Some(Value::String("abab".into()))
    );
    assert_eq!(repeat.errors.len(), 2);
    assert!(repeat.errors.iter().all(|error| matches!(&error.error, RuntimeError::InvalidValue { actual: Value::Number(n), constraint } if !n.is_finite() && !constraint.is_empty())));
    for id in &ids[..7] {
        assert!(output(&result, id).errors.is_empty(), "{id}");
    }
    let clamped = engine
        .evaluate(&request(1, &["repeat"], &[("n", numbers(&[10_001.0]))]))
        .unwrap();
    let Some(Value::String(text)) = row_value(&output(&clamped, "repeat").column, 0) else {
        panic!()
    };
    assert_eq!(text.len(), 20_000);
}

#[test]
fn dynamic_operation_mismatches_are_row_errors_and_branches_can_skip_them() {
    let engine = engine(vec![
        input("x", ValueType::Unknown),
        input("skip", ValueType::Boolean),
        formula("length", "length(prop(\"x\"))"),
        formula("skip_length", "if(prop(\"skip\"), 99, prop(\"length\"))"),
        formula("sum", "sum(prop(\"x\"))"),
        formula("flat", "flat(prop(\"x\"))"),
    ]);
    let result = engine
        .evaluate(&request(
            3,
            &["length", "skip_length", "sum", "flat"],
            &[
                (
                    "x",
                    dynamic(&[
                        Value::String("abc".into()),
                        Value::Number(1.0),
                        Value::List(vec![Some(Value::String("x".into()))]),
                    ]),
                ),
                ("skip", booleans(&[false, true, false])),
            ],
        ))
        .unwrap();
    let length = output(&result, "length");
    assert_eq!(row_value(&length.column, 0), Some(Value::Number(3.0)));
    assert_eq!(row_value(&length.column, 1), None);
    assert_eq!(length.errors.len(), 1);
    assert!(matches!(
        &length.errors[0].error,
        RuntimeError::InvalidValueType {
            actual: ValueType::Number,
            ..
        }
    ));
    assert!(output(&result, "skip_length").errors.is_empty());
    assert_eq!(
        output(&result, "skip_length").column,
        numbers(&[3.0, 99.0, 1.0])
    );
    assert_eq!(output(&result, "sum").errors.len(), 2);
    assert_eq!(output(&result, "flat").errors.len(), 2);
}

#[test]
fn runtime_snapshot_date_ranges_and_output_types_survive_updates() {
    let mut engine = engine(vec![
        input("date", ValueType::Date),
        formula("now", "now()"),
        formula("today", "today()"),
        formula("year", "year(prop(\"date\"))"),
        formula("saved", "1"),
    ]);
    let dates = Column::Date(ColumnData {
        values: vec![0, -62_135_596_800_001, 253_402_300_799_999],
        validity: NullBuffer::new_valid(3),
    });
    let mut input = request(3, &["now", "today", "year", "saved"], &[("date", dates)]);
    input.runtime.now = 0;
    let result = engine.evaluate(&input).unwrap();
    assert_eq!(
        row_value(&output(&result, "now").column, 0),
        Some(Value::Date(0))
    );
    assert_eq!(
        row_value(&output(&result, "today").column, 0),
        Some(Value::Date(-28_800_000))
    );
    assert_eq!(
        row_value(&output(&result, "year").column, 0),
        Some(Value::Number(1970.0))
    );
    let errors = &output(&result, "year").errors;
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|error| matches!(error.error, RuntimeError::DateOutOfRange))
    );
    assert_eq!(
        snapshot(&result),
        snapshot(&engine.evaluate(&input).unwrap())
    );
    engine.upsert(formula("saved", "\"new\"")).unwrap();
    assert_eq!(output(&result, "saved").output_type, ValueType::Number);
    assert_eq!(
        output(&engine.evaluate(&input).unwrap(), "saved").output_type,
        ValueType::String
    );
    assert!(
        matches!(engine.property(&"saved".into()), Some(PropertyState::Formula(state)) if matches!(state.status, FormulaStatus::Ready { output_type: ValueType::String }))
    );
}

#[test]
fn invalid_unused_inputs_and_ids_are_rejected_before_execution() {
    let engine = engine(vec![
        input("unused", ValueType::Number),
        formula("fails", "test(\"x\", \"[\")"),
    ]);
    let mut input = request(1, &["fails"], &[]);
    assert_eq!(
        engine.evaluate(&input),
        Err(EvaluateInputError::MissingInputs {
            ids: vec!["unused".into()]
        })
    );
    input.columns.insert("unused".into(), strings(&["wrong"]));
    assert_eq!(
        engine.evaluate(&input),
        Err(EvaluateInputError::InvalidColumnType {
            id: "unused".into(),
            expected: ColumnKind::Number,
            actual: ColumnKind::String,
        })
    );
    input.columns.insert("unused".into(), numbers(&[1.0, 2.0]));
    assert_eq!(
        engine.evaluate(&input),
        Err(EvaluateInputError::InvalidColumnLength {
            id: "unused".into(),
            expected: 1,
            values_len: 2,
            validity_len: 2
        })
    );
    input.columns.insert("unused".into(), numbers(&[1.0]));
    input.formula_ids = vec!["unused".into()];
    assert_eq!(
        engine.evaluate(&input),
        Err(EvaluateInputError::InvalidFormulaId {
            id: "unused".into()
        })
    );
    input.formula_ids = vec!["fails".into()];
    input.row_ids[0] = "".into();
    assert_eq!(
        engine.evaluate(&input),
        Err(EvaluateInputError::EmptyRowId { row_index: 0 })
    );
}

#[test]
fn nested_union_validation_and_zero_row_column_kinds_use_declared_types() {
    let union = ValueType::Union(vec![
        ValueType::List(Box::new(ValueType::Number)),
        ValueType::List(Box::new(ValueType::String)),
    ]);
    let engine = engine(vec![
        input("x", union.clone()),
        formula("copy", "prop(\"x\")"),
        formula("empty", "[]"),
    ]);
    let bad = request(
        1,
        &["copy"],
        &[(
            "x",
            dynamic(&[Value::List(vec![
                Some(Value::Number(1.0)),
                Some(Value::String("x".into())),
            ])]),
        )],
    );
    assert!(
        matches!(engine.evaluate(&bad), Err(EvaluateInputError::InvalidValueType { expected, element_path, row_index: 0, .. }) if expected == union && element_path.is_empty())
    );
    let result = engine
        .evaluate(&request(0, &["copy", "empty"], &[("x", dynamic(&[]))]))
        .unwrap();
    assert_eq!(output(&result, "copy").output_type, union);
    assert!(
        matches!(&output(&result, "copy").column, Column::Union(data) if data.values.is_empty() && data.validity.is_empty())
    );
    assert_eq!(
        output(&result, "empty").output_type,
        ValueType::List(Box::new(ValueType::Unknown))
    );
    assert!(
        matches!(&output(&result, "empty").column, Column::List(data) if data.values.is_empty())
    );
}

#[test]
fn long_dependency_chain_evaluates_without_recursive_graph_execution() {
    let count = 1_000;
    let mut definitions = vec![formula("f0000", "1")];
    for index in 1..count {
        definitions.push(formula(
            &format!("f{index:04}"),
            &format!("prop(\"f{:04}\") + 1", index - 1),
        ));
    }
    let engine = engine(definitions);
    let result = engine.evaluate(&request(1, &["f0999"], &[])).unwrap();
    assert_eq!(output(&result, "f0999").column, numbers(&[1_000.0]));
}

#[test]
fn null_placeholders_and_union_storage_do_not_change_output_types() {
    let ty = ValueType::Union(vec![
        ValueType::List(Box::new(ValueType::Number)),
        ValueType::List(Box::new(ValueType::String)),
    ]);
    let engine = engine(vec![
        input("x", ty.clone()),
        formula("copy", "prop(\"x\")"),
        formula("length", "length(prop(\"copy\"))"),
    ]);
    let column = Column::Union(ColumnData {
        values: vec![
            Value::List(vec![Some(Value::Number(1.0)), None]),
            Value::List(vec![Some(Value::String("x".into()))]),
            // Null placeholders are not values and need not match the declaration.
            Value::Boolean(true),
        ],
        validity: NullBuffer::from(vec![true, true, false]),
    });
    let result = engine
        .evaluate(&request(3, &["copy", "length"], &[("x", column.clone())]))
        .unwrap();
    assert_eq!(output(&result, "copy").output_type, ty);
    assert!(matches!(output(&result, "copy").column, Column::Union(_)));
    for row in 0..3 {
        assert_eq!(
            row_value(&output(&result, "copy").column, row),
            row_value(&column, row)
        );
    }
    assert_eq!(
        row_value(&output(&result, "length").column, 0),
        Some(Value::Number(2.0))
    );
    assert_eq!(
        row_value(&output(&result, "length").column, 1),
        Some(Value::Number(1.0))
    );
    assert_eq!(row_value(&output(&result, "length").column, 2), None);
    assert!(output(&result, "length").errors.is_empty());
}

#[test]
fn list_traversal_propagates_dependency_errors_only_when_read() {
    let engine = engine(vec![
        formula("bad", "test(\"x\", \"[\")"),
        formula("empty_map", "map([], prop(\"bad\"))"),
        formula("map", "map([1], if(current == 1, true, prop(\"bad\")))"),
        formula(
            "find",
            "find([1, 2], if(current == 1, true, prop(\"bad\")))",
        ),
        formula("fail", "map([2], if(current == 1, true, prop(\"bad\")))"),
    ]);
    let result = engine
        .evaluate(&request(2, &["empty_map", "map", "find", "fail"], &[]))
        .unwrap();
    for id in ["empty_map", "map", "find"] {
        assert!(output(&result, id).errors.is_empty(), "{id}");
    }
    assert_eq!(
        row_value(&output(&result, "find").column, 0),
        Some(Value::Number(1.0))
    );
    assert_eq!(output(&result, "fail").errors.len(), 2);
    assert!(
        output(&result, "fail")
            .errors
            .iter()
            .all(|error| error.origin_formula_id == PropertyId::from("bad"))
    );
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Number(value) if value.is_nan() => "NaN".into(),
        Value::Number(value) if *value == f64::INFINITY => "Infinity".into(),
        Value::Number(value) if *value == f64::NEG_INFINITY => "-Infinity".into(),
        Value::Number(value) if *value == 0.0 && value.is_sign_negative() => "-0".into(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => format!("{value:?}"),
        Value::Boolean(value) => value.to_string(),
        Value::Date(value) => format!("date({value})"),
        Value::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|value| value
                    .as_ref()
                    .map(render_value)
                    .unwrap_or_else(|| "null".into()))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn snapshot(result: &EvaluateResult) -> String {
    let mut text = String::new();
    let mut formulas = result.formulas.iter().collect::<Vec<_>>();
    formulas.sort_by_key(|(id, _)| &id.0);
    for (id, output) in formulas {
        match output {
            Err(error) => writeln!(text, "{}: {error:?}", id.0).unwrap(),
            Ok(output) => {
                writeln!(text, "{}: {:?}", id.0, output.output_type).unwrap();
                let len = match &output.column {
                    Column::Number(data) => data.values.len(),
                    Column::String(data) => data.values.len(),
                    Column::Boolean(data) => data.values.len(),
                    Column::Date(data) => data.values.len(),
                    Column::List(data) => data.values.len(),
                    Column::Union(data) => data.values.len(),
                };
                for row in 0..len {
                    writeln!(
                        text,
                        "  row {row}: {}",
                        row_value(&output.column, row)
                            .as_ref()
                            .map(render_value)
                            .unwrap_or_else(|| "null".into())
                    )
                    .unwrap();
                }
                for error in &output.errors {
                    writeln!(
                        text,
                        "  error row {} origin {}: {:?}",
                        error.row_index, error.origin_formula_id.0, error.error
                    )
                    .unwrap();
                }
            }
        }
    }
    text
}

#[test]
fn public_evaluation_snapshot_is_reproducible() {
    let engine = engine(vec![
        input("skip", ValueType::Boolean),
        formula("source", "test(\"x\", \"[\")"),
        formula("conditional", "if(prop(\"skip\"), true, prop(\"source\"))"),
        formula("nan", "0 / 0"),
        formula("infinity", "1 / 0"),
        formula("zero", "-0"),
        formula("null", "empty()"),
        formula("list", "map([1, empty(), 2], current)"),
        formula("not_ready", "prop(\"missing\")"),
    ]);
    let ids = [
        "source",
        "conditional",
        "nan",
        "infinity",
        "zero",
        "null",
        "list",
        "not_ready",
    ];
    let input = request(2, &ids, &[("skip", booleans(&[true, false]))]);
    let actual = snapshot(&engine.evaluate(&input).unwrap());
    assert_eq!(actual, snapshot(&engine.evaluate(&input).unwrap()));
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/evaluation.snap");
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, &actual).unwrap();
    }
    assert_eq!(
        actual,
        std::fs::read_to_string(path)
            .expect("run BLESS=1 cargo test -p formula_engine --test evaluation")
    );
}
