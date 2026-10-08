use std::collections::HashMap;

use formula_engine::{
    Column, ColumnData, DateValue, EvaluateInput, FormulaDefinition, FormulaEngine, FormulaOutput,
    FormulaSchema, NullBuffer, PropertyDefinition, RuntimeContext, Value, ValueType,
};

fn evaluate(
    expression: &str,
    columns: HashMap<formula_engine::PropertyId, Column>,
    rows: usize,
) -> FormulaOutput {
    let mut properties: Vec<_> = columns
        .keys()
        .map(|id| PropertyDefinition::Input {
            id: id.clone(),
            ty: match &columns[id] {
                Column::Union(_) => ValueType::Union(vec![ValueType::Date, ValueType::Number]),
                Column::List(_) => ValueType::List(Box::new(ValueType::Date)),
                Column::Boolean(_) => ValueType::Boolean,
                _ => ValueType::Date,
            },
        })
        .collect();
    properties.push(PropertyDefinition::Formula(FormulaDefinition {
        id: "result".into(),
        expression: expression.into(),
    }));
    let engine = FormulaEngine::new(FormulaSchema { properties }).unwrap();
    let mut result = engine
        .evaluate(&EvaluateInput {
            row_ids: (0..rows).map(|row| format!("row-{row}").into()).collect(),
            columns,
            runtime: RuntimeContext {
                now: 1_790_244_000_000,
                time_zone: "+08:00".into(),
            },
            formula_ids: vec!["result".into()],
        })
        .unwrap();
    result
        .formulas
        .remove(&"result".into())
        .unwrap()
        .unwrap_or_else(|error| panic!("{expression}: {error:?}"))
}

fn dates(values: Vec<DateValue>, validity: Vec<bool>) -> Column {
    Column::DateValue(ColumnData {
        values,
        validity: NullBuffer::from(validity),
    })
}

#[test]
fn saved_date_ranges_preserve_raw_endpoints_and_hidden_time_metadata() {
    let date = DateValue {
        start: 1_790_266_800_000,
        end: Some(1_790_407_200_000),
        include_time: false,
    };
    let column = dates(
        vec![
            date,
            DateValue {
                start: i64::MAX,
                end: Some(i64::MIN),
                include_time: true,
            },
        ],
        vec![true, false],
    );
    let result = evaluate(
        r#"prop("stay")"#,
        HashMap::from([("stay".into(), column)]),
        2,
    );
    assert_eq!(result.output_type, ValueType::Date);
    assert!(result.errors.is_empty());
    let Column::DateValue(output) = result.column else {
        panic!("range metadata must survive");
    };
    assert_eq!(output.values[0], date);
    assert!(output.validity.is_valid(0));
    assert!(output.validity.is_null(1));
}

#[test]
fn range_construction_and_endpoint_selection_accept_both_date_representations() {
    let range = DateValue {
        start: 2000,
        end: Some(1000),
        include_time: false,
    };
    for (expression, expected) in [
        (
            r#"dateStart(prop("date"))"#,
            DateValue {
                start: 2000,
                end: None,
                include_time: false,
            },
        ),
        (
            r#"dateEnd(prop("date"))"#,
            DateValue {
                start: 1000,
                end: None,
                include_time: false,
            },
        ),
        (
            r#"dateRange(prop("date"), empty())"#,
            DateValue {
                start: 2000,
                end: None,
                include_time: false,
            },
        ),
        (
            r#"dateRange(prop("date"), fromTimestamp(0))"#,
            DateValue {
                start: 2000,
                end: Some(0),
                include_time: true,
            },
        ),
    ] {
        let result = evaluate(
            expression,
            HashMap::from([("date".into(), dates(vec![range], vec![true]))]),
            1,
        );
        assert!(result.errors.is_empty(), "{expression}");
        assert_eq!(
            result.column,
            dates(vec![expected], vec![true]),
            "{expression}"
        );
    }
    for expression in ["dateStart(fromTimestamp(0))", "dateEnd(fromTimestamp(0))"] {
        let result = evaluate(expression, HashMap::new(), 1);
        assert_eq!(
            result.column,
            Column::Date(ColumnData {
                values: vec![0],
                validity: NullBuffer::new_valid(1)
            })
        );
    }
    let result = evaluate("dateRange(empty(), fromTimestamp(0))", HashMap::new(), 1);
    let Column::Date(column) = result.column else {
        panic!("all-null dates use legacy storage");
    };
    assert!(column.validity.is_null(0));
}

#[test]
fn shifting_dates_preserves_endpoints_and_between_observes_hidden_times() {
    // AppFlowy #9042: Sept 24 18:00 to Sept 26 09:00 at UTC+08:00.
    let date = DateValue {
        start: 1_790_244_000_000,
        end: Some(1_790_384_400_000),
        include_time: false,
    };
    let source = || HashMap::from([("stay".into(), dates(vec![date], vec![true]))]);
    for (expression, expected) in [
        (
            r#"dateAdd(prop("stay"), 1, "days")"#,
            DateValue {
                start: 1_790_330_400_000,
                end: Some(1_790_470_800_000),
                ..date
            },
        ),
        (
            r#"dateSubtract(prop("stay"), 1, "hours")"#,
            DateValue {
                start: 1_790_240_400_000,
                end: Some(1_790_380_800_000),
                ..date
            },
        ),
    ] {
        let result = evaluate(expression, source(), 1);
        assert!(result.errors.is_empty());
        assert_eq!(result.column, dates(vec![expected], vec![true]));
    }
    for (include_time, expected_days, expected_hours) in [(false, 2.0, 48.0), (true, 1.0, 39.0)] {
        for (unit, expected) in [("days", expected_days), ("hours", expected_hours)] {
            let result = evaluate(
                &format!(
                    "dateBetween(dateEnd(prop(\"stay\")), dateStart(prop(\"stay\")), \"{unit}\")"
                ),
                HashMap::from([(
                    "stay".into(),
                    dates(
                        vec![DateValue {
                            include_time,
                            ..date
                        }],
                        vec![true],
                    ),
                )]),
                1,
            );
            assert_eq!(
                result.column,
                Column::Number(ColumnData {
                    values: vec![expected],
                    validity: NullBuffer::new_valid(1)
                })
            );
        }
    }
}

#[test]
fn constructors_and_formatting_preserve_date_only_and_range_visibility() {
    for (expression, expected) in [
        (
            "today()",
            DateValue {
                start: 1_790_179_200_000,
                end: None,
                include_time: false,
            },
        ),
        (
            r#"parseDate("2026-09-24")"#,
            DateValue {
                start: 1_790_179_200_000,
                end: None,
                include_time: false,
            },
        ),
    ] {
        let result = evaluate(expression, HashMap::new(), 1);
        assert_eq!(
            result.column,
            dates(vec![expected], vec![true]),
            "{expression}"
        );
    }
    for (expression, expected) in [
        (
            r#"format(fromTimestamp(1790244000000))"#,
            "September 24, 2026 18:00",
        ),
        (r#"format(parseDate("2026-09-24"))"#, "September 24, 2026"),
        (
            r#"format(dateRange(parseDate("2026-09-24"), parseDate("2026-09-26")))"#,
            "September 24, 2026 → September 26, 2026",
        ),
        (
            r#"format(dateRange(parseDate("2026-09-24T18:00:00"), parseDate("2026-09-26T09:00:00")))"#,
            "September 24, 2026 18:00 → September 26, 2026 09:00",
        ),
    ] {
        let result = evaluate(expression, HashMap::new(), 1);
        assert!(
            result.errors.is_empty(),
            "{expression}: {:?}",
            result.errors
        );
        assert_eq!(
            result.column,
            Column::String(ColumnData {
                values: vec![expected.into()],
                validity: NullBuffer::new_valid(1)
            }),
            "{expression}"
        );
    }
    for (expression, expected) in [
        (r#"join([fromTimestamp(0)], ",")"#, "0"),
        (r#"join([parseDate("1970-01-01")], ",")"#, "-28800000"),
        (
            r#"join([dateRange(fromTimestamp(0), fromTimestamp(60000))], ",")"#,
            "0 → 60000",
        ),
        (
            r#"format([dateRange(fromTimestamp(0), fromTimestamp(60000))])"#,
            "[0 → 60000]",
        ),
    ] {
        let result = evaluate(expression, HashMap::new(), 1);
        assert!(
            result.errors.is_empty(),
            "{expression}: {:?}",
            result.errors
        );
        assert_eq!(
            result.column,
            Column::String(ColumnData {
                values: vec![expected.into()],
                validity: NullBuffer::new_valid(1)
            }),
            "{expression}"
        );
    }
}

#[test]
fn date_equality_uses_endpoints_ordering_uses_start_and_lists_preserve_metadata() {
    for (expression, expected) in [
        (
            r#"parseDate("1970-01-01") == fromTimestamp(-28800000)"#,
            true,
        ),
        (
            r#"equal(parseDate("1970-01-01"), fromTimestamp(-28800000))"#,
            true,
        ),
        (
            r#"[parseDate("1970-01-01")] == [fromTimestamp(-28800000)]"#,
            true,
        ),
        (
            r#"dateRange(fromTimestamp(0), fromTimestamp(60000)) == fromTimestamp(0)"#,
            false,
        ),
        (
            r#"dateRange(fromTimestamp(0), fromTimestamp(60000)) <= dateRange(fromTimestamp(0), fromTimestamp(120000))"#,
            true,
        ),
        (
            r#"includes([parseDate("1970-01-01")], fromTimestamp(-28800000))"#,
            true,
        ),
    ] {
        let result = evaluate(expression, HashMap::new(), 1);
        assert!(result.errors.is_empty());
        assert_eq!(
            result.column,
            Column::Boolean(ColumnData {
                values: vec![expected],
                validity: NullBuffer::new_valid(1)
            }),
            "{expression}"
        );
    }
    let result = evaluate(
        r#"length(unique([parseDate("1970-01-01"), fromTimestamp(-28800000)]))"#,
        HashMap::new(),
        1,
    );
    assert_eq!(
        result.column,
        Column::Number(ColumnData {
            values: vec![1.0],
            validity: NullBuffer::new_valid(1)
        })
    );
}

#[test]
fn every_consumed_range_endpoint_checks_utc_and_runtime_local_boundaries() {
    for date in [
        DateValue {
            start: 0,
            end: Some(i64::MAX),
            include_time: true,
        },
        // UTC is in year 9999; UTC+08:00 puts the end in year 10000.
        DateValue {
            start: 0,
            end: Some(253_402_300_799_999),
            include_time: false,
        },
    ] {
        for expression in [
            r#"minute(prop("date"))"#,
            r#"hour(prop("date"))"#,
            r#"day(prop("date"))"#,
            r#"date(prop("date"))"#,
            r#"week(prop("date"))"#,
            r#"month(prop("date"))"#,
            r#"year(prop("date"))"#,
            r#"timestamp(prop("date"))"#,
            r#"formatDate(prop("date"), "YYYY-MM-DD")"#,
            r#"format(prop("date"))"#,
            r#"dateStart(prop("date"))"#,
            r#"dateEnd(prop("date"))"#,
            r#"dateAdd(prop("date"), 0, "days")"#,
            r#"dateSubtract(prop("date"), 0, "months")"#,
            r#"dateBetween(prop("date"), now(), "hours")"#,
            r#"dateRange(prop("date"), now())"#,
        ] {
            let result = evaluate(
                expression,
                HashMap::from([("date".into(), dates(vec![date], vec![true]))]),
                1,
            );
            assert_eq!(result.errors.len(), 1, "{expression}");
            assert_eq!(
                result.errors[0].error,
                formula_engine::RuntimeError::DateOutOfRange,
                "{expression}"
            );
        }
    }
}

#[test]
fn mixed_and_nested_dates_preserve_metadata_and_null_batches_have_stable_storage() {
    let rich = DateValue {
        start: 60000,
        end: Some(0),
        include_time: false,
    };
    let result = evaluate(
        r#"if(prop("flag"), prop("rich"), prop("legacy"))"#,
        HashMap::from([
            (
                "flag".into(),
                Column::Boolean(ColumnData {
                    values: vec![true, false],
                    validity: NullBuffer::new_valid(2),
                }),
            ),
            ("rich".into(), dates(vec![rich; 2], vec![true; 2])),
            (
                "legacy".into(),
                Column::Date(ColumnData {
                    values: vec![120000; 2],
                    validity: NullBuffer::new_valid(2),
                }),
            ),
        ]),
        2,
    );
    assert_eq!(
        result.column,
        dates(
            vec![
                rich,
                DateValue {
                    start: 120000,
                    end: None,
                    include_time: true
                }
            ],
            vec![true; 2]
        )
    );

    let nested = vec![
        Some(Value::DateValue(rich)),
        None,
        Some(Value::Date(120000)),
    ];
    let column = Column::List(ColumnData {
        values: vec![nested],
        validity: NullBuffer::new_valid(1),
    });
    let result = evaluate(
        r#"map(prop("dates"), current)"#,
        HashMap::from([("dates".into(), column.clone())]),
        1,
    );
    assert_eq!(result.column, column);
    let column = Column::Union(ColumnData {
        values: vec![Value::DateValue(rich), Value::Date(120000)],
        validity: NullBuffer::new_valid(2),
    });
    let result = evaluate(
        r#"prop("dates")"#,
        HashMap::from([("dates".into(), column.clone())]),
        2,
    );
    assert_eq!(result.column, column);

    for count in [0, 1] {
        let result = evaluate(
            r#"prop("date")"#,
            HashMap::from([("date".into(), dates(vec![rich; count], vec![false; count]))]),
            count,
        );
        assert_eq!(
            result.column,
            Column::Date(ColumnData {
                values: vec![0; count],
                validity: NullBuffer::new_null(count)
            })
        );
    }
    let result = evaluate(
        r#"if(false, dateEnd(prop("date")), now())"#,
        HashMap::from([(
            "date".into(),
            dates(
                vec![DateValue {
                    end: Some(i64::MAX),
                    ..rich
                }],
                vec![true],
            ),
        )]),
        1,
    );
    assert!(result.errors.is_empty());
}
