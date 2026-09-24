//! Executable reference for the planned input contract; evaluation is not implemented here.

use std::fmt::Write;

use formula_engine::ValueType;

include!("support/evaluation_input_contract.h.rs");

#[test]
fn planned_input_reference_transcript() {
    use ValueType as T;

    let mut transcript = String::from("accepts\n");
    let cases = [
        ("number", T::Number, Some(Value::Number(1.0)), true),
        ("string", T::String, Some(Value::String("x".into())), true),
        ("boolean", T::Boolean, Some(Value::Boolean(false)), true),
        ("date", T::Date, Some(Value::Date(0)), true),
        (
            "wrong scalar",
            T::Number,
            Some(Value::String("1".into())),
            false,
        ),
        ("absent scalar", T::Number, None, true),
        ("absent empty union", T::Union(vec![]), None, true),
        ("absent unknown", T::Unknown, None, true),
        (
            "unknown scalar",
            T::Unknown,
            Some(Value::Boolean(true)),
            true,
        ),
        (
            "unknown list",
            T::Unknown,
            Some(Value::List(vec![Some(Value::String("x".into()))])),
            true,
        ),
        (
            "list with absent element",
            T::List(Box::new(T::Number)),
            Some(Value::List(vec![Some(Value::Number(2.0)), None])),
            true,
        ),
        (
            "list with wrong element",
            T::List(Box::new(T::Number)),
            Some(Value::List(vec![None, Some(Value::String("x".into()))])),
            false,
        ),
        (
            "nested wrong element",
            T::List(Box::new(T::List(Box::new(T::Number)))),
            Some(Value::List(vec![Some(Value::List(vec![Some(
                Value::String("x".into()),
            )]))])),
            false,
        ),
        (
            "empty list",
            T::List(Box::new(T::Number)),
            Some(Value::List(vec![])),
            true,
        ),
        ("absent list", T::List(Box::new(T::Number)), None, true),
        (
            "list with only absent element",
            T::List(Box::new(T::Number)),
            Some(Value::List(vec![None])),
            true,
        ),
        (
            "list of union accepts mixed elements",
            T::List(Box::new(T::Union(vec![T::Number, T::String]))),
            Some(Value::List(vec![
                Some(Value::Number(1.0)),
                Some(Value::String("x".into())),
            ])),
            true,
        ),
        (
            "union of lists rejects mixed elements",
            T::Union(vec![
                T::List(Box::new(T::Number)),
                T::List(Box::new(T::String)),
            ]),
            Some(Value::List(vec![
                Some(Value::Number(1.0)),
                Some(Value::String("x".into())),
            ])),
            false,
        ),
        (
            "union second member",
            T::Union(vec![T::Number, T::String]),
            Some(Value::String("x".into())),
            true,
        ),
        (
            "union excludes other kind",
            T::Union(vec![T::Number, T::String]),
            Some(Value::Boolean(true)),
            false,
        ),
        (
            "empty union excludes present value",
            T::Union(vec![]),
            Some(Value::Number(1.0)),
            false,
        ),
    ];

    for (name, ty, value, expected) in cases {
        let actual = accepts(&ty, value.as_ref());
        assert_eq!(actual, expected, "{name}");
        writeln!(transcript, "{name}: {actual}").unwrap();
    }

    transcript.push_str("column kind\n");
    let cases = [
        ("number", T::Number, ColumnKind::Number),
        ("string", T::String, ColumnKind::String),
        ("boolean", T::Boolean, ColumnKind::Boolean),
        ("date", T::Date, ColumnKind::Date),
        ("list", T::List(Box::new(T::Number)), ColumnKind::List),
        ("unknown", T::Unknown, ColumnKind::Union),
        ("empty union", T::Union(vec![]), ColumnKind::Union),
        (
            "single-member union",
            T::Union(vec![T::Number]),
            ColumnKind::Union,
        ),
    ];

    for (name, ty, expected) in cases {
        let actual = column_kind(&ty);
        assert_eq!(actual, expected, "{name}");
        writeln!(transcript, "{name}: {actual:?}").unwrap();
    }

    assert_eq!(transcript, include_str!("evaluation_input_contract.snap"));
}
