use std::collections::HashMap;

use super::{ValidatedRuntimeContext, validate_evaluate_input};
use crate::{
    Column, ColumnData, ColumnKind, EvaluateInput, EvaluateInputError, FormulaDefinition,
    NullBuffer, PropertyDefinition, PropertyId, RuntimeContext, Value, ValueType,
};

fn definitions(inputs: Vec<(&str, ValueType)>) -> Vec<PropertyDefinition> {
    inputs
        .into_iter()
        .map(|(id, ty)| PropertyDefinition::Input { id: id.into(), ty })
        .chain([PropertyDefinition::Formula(FormulaDefinition {
            id: "f".into(),
            expression: "1".into(),
        })])
        .collect()
}

fn request(rows: usize, columns: Vec<(&str, Column)>) -> EvaluateInput {
    EvaluateInput {
        row_ids: (0..rows)
            .map(|index| format!("row-{index}").into())
            .collect(),
        columns: columns
            .into_iter()
            .map(|(id, column)| (id.into(), column))
            .collect(),
        runtime: RuntimeContext {
            now: 0,
            time_zone: "+00:00".into(),
        },
        formula_ids: vec!["f".into()],
    }
}

fn data<T>(values: Vec<T>) -> ColumnData<T> {
    let validity = NullBuffer::new_valid(values.len());
    ColumnData { values, validity }
}

fn validate(
    definitions: &[PropertyDefinition],
    input: &EvaluateInput,
) -> Result<ValidatedRuntimeContext, EvaluateInputError> {
    validate_evaluate_input(definitions, input)
}

fn list(inner: ValueType) -> ValueType {
    ValueType::List(Box::new(inner))
}

fn type_for_kind(kind: ColumnKind) -> ValueType {
    match kind {
        ColumnKind::Number => ValueType::Number,
        ColumnKind::String => ValueType::String,
        ColumnKind::Boolean => ValueType::Boolean,
        ColumnKind::Date => ValueType::Date,
        ColumnKind::List => list(ValueType::Number),
        ColumnKind::Union => ValueType::Union(vec![ValueType::Number]),
    }
}

fn null_column(kind: ColumnKind, values_len: usize, validity_len: usize) -> Column {
    let validity = NullBuffer::new_null(validity_len);
    match kind {
        ColumnKind::Number => Column::Number(ColumnData {
            values: vec![f64::NAN; values_len],
            validity,
        }),
        ColumnKind::String => Column::String(ColumnData {
            values: vec![String::new(); values_len],
            validity,
        }),
        ColumnKind::Boolean => Column::Boolean(ColumnData {
            values: vec![false; values_len],
            validity,
        }),
        ColumnKind::Date => Column::Date(ColumnData {
            values: vec![i64::MAX; values_len],
            validity,
        }),
        ColumnKind::List => Column::List(ColumnData {
            values: vec![vec![Some(Value::String("invalid placeholder".into()))]; values_len],
            validity,
        }),
        ColumnKind::Union => Column::Union(ColumnData {
            values: vec![Value::String("invalid placeholder".into()); values_len],
            validity,
        }),
    }
}

#[test]
fn runtime_returns_seconds_and_rejects_inexact_offset_formats() {
    let definitions = definitions(vec![]);
    let mut input = request(0, vec![]);
    for (time_zone, expected) in [
        ("+00:00", 0),
        ("-00:00", 0),
        ("+08:00", 28_800),
        ("-00:01", -60),
        ("+23:59", 86_340),
        ("-23:59", -86_340),
        ("+01:30", 5_400),
    ] {
        input.runtime.time_zone = time_zone.into();
        assert_eq!(
            validate(&definitions, &input).unwrap(),
            ValidatedRuntimeContext {
                now: 0,
                time_zone_offset_seconds: expected,
            },
            "{time_zone}"
        );
    }

    for time_zone in [
        "",
        "UTC",
        "Asia/Singapore",
        "Z",
        "08:00",
        "+8:00",
        "+08:0",
        "+0800",
        "+08:00:00",
        " +08:00",
        "+08:00 ",
        "+08:00\n",
        "−08:00",
        "＋08:00",
        "+０８:00",
        "+08:٠٠",
        "+24:00",
        "-24:00",
        "+00:60",
        "-23:60",
        "+99:99",
        "+0a:00",
        "+00:a0",
        "+00;00",
    ] {
        input.runtime.time_zone = time_zone.into();
        assert_eq!(
            validate(&definitions, &input).unwrap_err(),
            EvaluateInputError::InvalidTimeZone {
                time_zone: time_zone.into(),
            },
            "{time_zone:?}"
        );
    }
}

#[test]
fn runtime_checks_both_utc_and_adjusted_local_gregorian_years() {
    let definitions = definitions(vec![]);
    let mut input = request(0, vec![]);
    // 0001-01-01T00:00:00.000Z and 9999-12-31T23:59:59.999Z.
    let first = -62_135_596_800_000;
    let last = 253_402_300_799_999;
    for (now, time_zone, accepted) in [
        (first, "+00:00", true),
        (last, "-00:00", true),
        (first - 1, "+00:01", false),
        (last + 1, "-00:01", false),
        (first, "-00:01", false),
        (last, "+00:01", false),
        (first + 59_999, "-00:01", false),
        (first + 60_000, "-00:01", true),
        (last - 59_999, "+00:01", false),
        (last - 60_000, "+00:01", true),
        (first, "+23:59", true),
        (last, "-23:59", true),
        (i64::MIN, "+23:59", false),
        (i64::MAX, "-23:59", false),
    ] {
        input.runtime = RuntimeContext {
            now,
            time_zone: time_zone.into(),
        };
        let result = validate(&definitions, &input);
        if accepted {
            assert_eq!(result.unwrap().now, now, "{now} {time_zone}");
        } else {
            assert_eq!(
                result.unwrap_err(),
                EvaluateInputError::InvalidNow { now },
                "{now} {time_zone}"
            );
        }
    }
}

#[test]
fn rows_require_nonempty_unique_ids_without_normalization() {
    let definitions = definitions(vec![]);
    let mut input = request(4, vec![]);
    input.row_ids = vec!["Å".into(), "Å".into(), "name".into(), "Name".into()];
    validate(&definitions, &input).unwrap();

    input.row_ids[2] = "".into();
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::EmptyRowId { row_index: 2 }
    );
    input.row_ids[2] = "Å".into();
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::DuplicateRowId { id: "Å".into() }
    );
}

#[test]
fn requests_require_formula_ids_and_allow_unready_formula_definitions() {
    let mut definitions = definitions(vec![("input", ValueType::Number)]);
    definitions.push(PropertyDefinition::Formula(FormulaDefinition {
        id: "NotReady".into(),
        expression: "1 +".into(),
    }));
    let mut input = request(0, vec![("input", Column::Number(data(vec![])))]);
    input.formula_ids = vec!["NotReady".into(), "f".into()];
    validate(&definitions, &input).unwrap();

    input.formula_ids.clear();
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::EmptyFormulaIds
    );
    for id in ["", "missing", "input", "F", "notready"] {
        input.formula_ids = vec!["f".into(), id.into()];
        assert_eq!(
            validate(&definitions, &input).unwrap_err(),
            EvaluateInputError::InvalidFormulaId { id: id.into() },
            "{id:?}"
        );
    }
    input.formula_ids = vec!["f".into(), "NotReady".into(), "f".into()];
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::DuplicateFormulaId { id: "f".into() }
    );
}

#[test]
fn all_input_ids_are_required_and_extra_ids_are_reported_in_sorted_sets() {
    let definitions = definitions(vec![
        ("z", ValueType::Number),
        ("Å", ValueType::Number),
        ("a", ValueType::Number),
        ("A", ValueType::Number),
    ]);
    let mut input = request(0, vec![("z", Column::Number(data(vec![])))]);
    input
        .columns
        .insert("extra".into(), Column::Number(data(vec![])));
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::MissingInputs {
            ids: vec!["A".into(), "a".into(), "Å".into()],
        }
    );
    for id in ["Å", "a", "A"] {
        input
            .columns
            .insert(id.into(), Column::Number(data(vec![])));
    }
    // Formula IDs and empty strings are also extra column IDs.
    for id in ["f", "", "extra"] {
        input
            .columns
            .insert(id.into(), Column::Number(data(vec![])));
    }
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::UnexpectedInputs {
            ids: vec!["".into(), "extra".into(), "f".into()],
        }
    );
}

#[test]
fn an_unused_input_still_checks_every_present_value() {
    // f does not reference either Input.
    let definitions = definitions(vec![
        ("unused", list(ValueType::Number)),
        ("other", ValueType::Number),
    ]);
    let input = request(
        2,
        vec![
            ("other", Column::Number(data(vec![1.0, 2.0]))),
            (
                "unused",
                Column::List(data(vec![
                    vec![Some(Value::Number(1.0))],
                    vec![Some(Value::Boolean(true))],
                ])),
            ),
        ],
    );
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::InvalidValueType {
            id: "unused".into(),
            row_index: 1,
            element_path: vec![0],
            expected: ValueType::Number,
            actual: ValueType::Boolean,
        }
    );
}

#[test]
fn zero_rows_and_all_null_columns_keep_the_declared_physical_kind() {
    let kinds = [
        ColumnKind::Number,
        ColumnKind::String,
        ColumnKind::Boolean,
        ColumnKind::Date,
        ColumnKind::List,
        ColumnKind::Union,
    ];
    let mut types: Vec<_> = kinds
        .into_iter()
        .map(|kind| (type_for_kind(kind), kind))
        .collect();
    types.extend([
        (ValueType::Unknown, ColumnKind::Union),
        (ValueType::Union(vec![]), ColumnKind::Union),
    ]);
    for rows in [0, 2] {
        for (ty, expected) in &types {
            let definitions = definitions(vec![("x", ty.clone())]);
            let input = request(rows, vec![("x", null_column(*expected, rows, rows))]);
            validate(&definitions, &input).unwrap();
            for actual in kinds {
                if actual == *expected {
                    continue;
                }
                let input = request(rows, vec![("x", null_column(actual, rows, rows))]);
                assert_eq!(
                    validate(&definitions, &input).unwrap_err(),
                    EvaluateInputError::InvalidColumnType {
                        id: "x".into(),
                        expected: *expected,
                        actual,
                    },
                    "{ty:?}, {rows} rows, {actual:?}"
                );
            }
        }
    }
}

#[test]
fn values_and_validity_lengths_are_independently_checked_for_every_kind() {
    for kind in [
        ColumnKind::Number,
        ColumnKind::String,
        ColumnKind::Boolean,
        ColumnKind::Date,
        ColumnKind::List,
        ColumnKind::Union,
    ] {
        let definitions = definitions(vec![("x", type_for_kind(kind))]);
        for (rows, values_len, validity_len) in [
            (2, 0, 0),
            (2, 1, 2),
            (2, 2, 1),
            (2, 3, 2),
            (2, 2, 3),
            (2, 3, 3),
            (0, 1, 0),
            (0, 0, 1),
        ] {
            let input = request(
                rows,
                vec![("x", null_column(kind, values_len, validity_len))],
            );
            assert_eq!(
                validate(&definitions, &input).unwrap_err(),
                EvaluateInputError::InvalidColumnLength {
                    id: "x".into(),
                    expected: rows,
                    values_len,
                    validity_len,
                },
                "{kind:?}"
            );
        }
    }
}

#[test]
fn nested_lists_preserve_nulls_empty_lists_and_unknown_members() {
    let definitions = definitions(vec![
        (
            "nested",
            list(list(ValueType::Union(vec![
                ValueType::Number,
                ValueType::String,
            ]))),
        ),
        ("unknown", list(ValueType::Unknown)),
        ("dynamic", ValueType::Unknown),
        ("empty_union", list(ValueType::Union(vec![]))),
    ]);
    let input = request(
        3,
        vec![
            (
                "nested",
                Column::List(data(vec![
                    vec![],
                    vec![None, Some(Value::List(vec![]))],
                    vec![Some(Value::List(vec![
                        Some(Value::Number(1.0)),
                        None,
                        Some(Value::String("x".into())),
                    ]))],
                ])),
            ),
            (
                "unknown",
                Column::List(data(vec![
                    vec![Some(Value::Boolean(true))],
                    vec![Some(Value::Date(i64::MIN))],
                    vec![Some(Value::List(vec![Some(Value::List(vec![None]))]))],
                ])),
            ),
            (
                "dynamic",
                Column::Union(data(vec![
                    Value::Number(f64::INFINITY),
                    Value::Date(i64::MAX),
                    Value::List(vec![None, Some(Value::String("x".into()))]),
                ])),
            ),
            (
                "empty_union",
                Column::List(data(vec![vec![], vec![None], vec![None, None]])),
            ),
        ],
    );
    validate(&definitions, &input).unwrap();
}

#[test]
fn nested_leaf_failure_reports_the_first_row_and_full_element_path() {
    let definitions = definitions(vec![("x", list(list(ValueType::Number)))]);
    let input = request(
        3,
        vec![(
            "x",
            Column::List(data(vec![
                vec![None],
                vec![
                    None,
                    Some(Value::List(vec![None, Some(Value::String("bad".into()))])),
                ],
                vec![Some(Value::Boolean(true))],
            ])),
        )],
    );
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::InvalidValueType {
            id: "x".into(),
            row_index: 1,
            element_path: vec![1, 1],
            expected: ValueType::Number,
            actual: ValueType::String,
        }
    );
}

#[test]
fn failed_union_reports_its_own_position_and_complete_actual_structure() {
    let union = ValueType::Union(vec![list(ValueType::Number), list(ValueType::String)]);
    let mixed = Value::List(vec![
        Some(Value::Number(1.0)),
        None,
        Some(Value::String("x".into())),
        Some(Value::Number(2.0)),
    ]);
    let actual = list(ValueType::Union(vec![ValueType::Number, ValueType::String]));
    let root_definitions = definitions(vec![("x", union.clone())]);
    let root = request(1, vec![("x", Column::Union(data(vec![mixed.clone()])))]);
    assert_eq!(
        validate(&root_definitions, &root).unwrap_err(),
        EvaluateInputError::InvalidValueType {
            id: "x".into(),
            row_index: 0,
            element_path: vec![],
            expected: union.clone(),
            actual: actual.clone(),
        }
    );

    let nested_definitions = definitions(vec![("x", list(list(union.clone())))]);
    let nested = request(
        1,
        vec![(
            "x",
            Column::List(data(vec![vec![Some(Value::List(vec![None, Some(mixed)]))]])),
        )],
    );
    assert_eq!(
        validate(&nested_definitions, &nested).unwrap_err(),
        EvaluateInputError::InvalidValueType {
            id: "x".into(),
            row_index: 0,
            element_path: vec![0, 1],
            expected: union,
            actual,
        }
    );
}

#[test]
fn union_members_are_tried_as_complete_types_and_unknown_accepts_any_value() {
    let definitions = definitions(vec![
        (
            "lists",
            ValueType::Union(vec![list(ValueType::Number), list(ValueType::String)]),
        ),
        (
            "members",
            list(ValueType::Union(vec![ValueType::Number, ValueType::String])),
        ),
        (
            "unknown",
            ValueType::Union(vec![ValueType::Number, ValueType::Unknown]),
        ),
    ]);
    let input = request(
        2,
        vec![
            (
                "lists",
                Column::Union(data(vec![
                    Value::List(vec![Some(Value::String("x".into())), None]),
                    Value::List(vec![]),
                ])),
            ),
            (
                "members",
                Column::List(data(vec![
                    vec![Some(Value::Number(1.0)), Some(Value::String("x".into()))],
                    vec![None],
                ])),
            ),
            (
                "unknown",
                Column::Union(data(vec![
                    Value::Boolean(true),
                    Value::List(vec![Some(Value::Date(i64::MAX)), None]),
                ])),
            ),
        ],
    );
    validate(&definitions, &input).unwrap();

    let definitions = self::definitions(vec![("x", ValueType::Union(vec![]))]);
    let input = request(
        1,
        vec![("x", Column::Union(data(vec![Value::List(vec![None])])))],
    );
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::InvalidValueType {
            id: "x".into(),
            row_index: 0,
            element_path: vec![],
            expected: ValueType::Union(vec![]),
            actual: list(ValueType::Unknown),
        }
    );
}

#[test]
fn binary64_special_values_and_unrestricted_date_inputs_pass_in_all_positions() {
    let numbers = vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0, 0.0];
    let bits: Vec<_> = numbers.iter().map(|number| number.to_bits()).collect();
    let dates = vec![
        i64::MIN,
        i64::MAX,
        -62_135_596_800_001,
        253_402_300_800_000,
        0,
    ];
    let definitions = definitions(vec![
        ("number", ValueType::Number),
        ("date", ValueType::Date),
        (
            "nested",
            list(ValueType::Union(vec![ValueType::Number, ValueType::Date])),
        ),
        (
            "union",
            ValueType::Union(vec![ValueType::Number, ValueType::Date]),
        ),
    ]);
    let input = request(
        5,
        vec![
            ("number", Column::Number(data(numbers.clone()))),
            ("date", Column::Date(data(dates.clone()))),
            (
                "nested",
                Column::List(data(
                    numbers
                        .iter()
                        .zip(&dates)
                        .map(|(&number, &date)| {
                            vec![Some(Value::Number(number)), None, Some(Value::Date(date))]
                        })
                        .collect(),
                )),
            ),
            (
                "union",
                Column::Union(data(vec![
                    Value::Number(f64::NAN),
                    Value::Number(f64::INFINITY),
                    Value::Number(-0.0),
                    Value::Date(i64::MIN),
                    Value::Date(i64::MAX),
                ])),
            ),
        ],
    );
    validate(&definitions, &input).unwrap();
    let Column::Number(column) = &input.columns[&PropertyId::from("number")] else {
        panic!("expected number input");
    };
    assert_eq!(
        column
            .values
            .iter()
            .map(|number| number.to_bits())
            .collect::<Vec<_>>(),
        bits
    );
}

#[test]
fn null_root_placeholders_are_ignored_including_sliced_validity_buffers() {
    let definitions = definitions(vec![
        ("union", ValueType::Union(vec![ValueType::Number])),
        ("list", list(ValueType::Number)),
    ]);
    let input = request(
        2,
        vec![
            (
                "union",
                Column::Union(ColumnData {
                    values: vec![Value::String("wrong".into()), Value::Number(3.0)],
                    validity: NullBuffer::from(vec![true, false, true, false]).slice(1, 2),
                }),
            ),
            (
                "list",
                Column::List(ColumnData {
                    values: vec![
                        vec![Some(Value::List(vec![Some(Value::Boolean(true))]))],
                        vec![None, Some(Value::Number(1.0))],
                    ],
                    validity: NullBuffer::from(vec![false, true]),
                }),
            ),
        ],
    );
    validate(&definitions, &input).unwrap();
}

#[test]
fn error_selection_is_independent_of_hash_map_and_definition_insertion_order() {
    let declarations = vec![
        ("z", ValueType::Number),
        ("a", list(ValueType::Number)),
        ("m", ValueType::Boolean),
    ];
    for reverse_definitions in [false, true] {
        let mut definitions = definitions(declarations.clone());
        if reverse_definitions {
            definitions.reverse();
        }
        for reverse_columns in [false, true] {
            for _ in 0..12 {
                let mut columns = vec![
                    ("z", Column::String(data(vec![]))),
                    ("m", Column::Boolean(data(vec![true]))),
                    (
                        "a",
                        Column::List(data(vec![vec![Some(Value::String("bad".into()))]])),
                    ),
                ];
                if reverse_columns {
                    columns.reverse();
                }
                let input = request(1, columns);
                assert_eq!(
                    validate(&definitions, &input).unwrap_err(),
                    EvaluateInputError::InvalidValueType {
                        id: "a".into(),
                        row_index: 0,
                        element_path: vec![0],
                        expected: ValueType::Number,
                        actual: ValueType::String,
                    }
                );
            }
        }
    }
}

#[test]
fn validation_priority_stays_fixed_when_multiple_errors_are_present() {
    let definitions = definitions(vec![("x", ValueType::Number)]);
    let mut input = request(1, vec![("extra", Column::Number(data(vec![])))]);
    input.runtime.now = i64::MAX;
    input.runtime.time_zone = "invalid".into();
    input.row_ids = vec!["".into()];
    input.formula_ids.clear();
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::InvalidNow { now: i64::MAX }
    );
    input.runtime.now = 0;
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::InvalidTimeZone {
            time_zone: "invalid".into()
        }
    );
    input.runtime.time_zone = "+00:00".into();
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::EmptyRowId { row_index: 0 }
    );
    input.row_ids = vec!["row".into()];
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::EmptyFormulaIds
    );
    input.formula_ids = vec!["missing".into()];
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::InvalidFormulaId {
            id: "missing".into()
        }
    );
    input.formula_ids = vec!["f".into(), "f".into()];
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::DuplicateFormulaId { id: "f".into() }
    );
    input.formula_ids = vec!["f".into()];
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::MissingInputs {
            ids: vec!["x".into()]
        }
    );
    input
        .columns
        .insert("x".into(), Column::String(data(vec![])));
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::UnexpectedInputs {
            ids: vec!["extra".into()]
        }
    );
    input.columns.remove(&PropertyId::from("extra"));
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::InvalidColumnType {
            id: "x".into(),
            expected: ColumnKind::Number,
            actual: ColumnKind::String
        }
    );
    input.columns = HashMap::from([("x".into(), Column::Number(data(vec![])))]);
    assert_eq!(
        validate(&definitions, &input).unwrap_err(),
        EvaluateInputError::InvalidColumnLength {
            id: "x".into(),
            expected: 1,
            values_len: 0,
            validity_len: 0
        }
    );
    input
        .columns
        .insert("x".into(), Column::Number(data(vec![1.0])));
    validate(&definitions, &input).unwrap();
}
