#[path = "support/builtin_eval.rs"]
mod builtin_eval;

use builtin_eval::{engine, output, request, value};
use formula_engine::{Value, ValueType};

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
fn list_callback_index_keeps_zero_row_batches_typed() {
    let source = "map([1,2], index)";
    let result = output(&engine(source), &request(0));
    assert_eq!(
        result.output_type,
        ValueType::List(Box::new(ValueType::Number)),
        "{source}"
    );
    assert!(result.errors.is_empty());
}
