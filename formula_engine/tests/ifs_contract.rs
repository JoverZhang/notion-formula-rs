#[path = "support/builtin_eval.rs"]
mod builtin_eval;

use builtin_eval::{engine, output, request, value};
use formula_engine::{
    Column, FormulaDefinition, FormulaEvaluationError, RuntimeError, Value, ValueType,
};

#[test]
fn ifs_requires_else_and_preserves_explicit_null_types_and_lazy_errors() {
    for source in [
        "ifs(true, 11)",
        "ifs(false, 11, false, 22)",
        "ifs(false, 11, false, 22, false, 33)",
    ] {
        let e = engine(source);
        let draft = e
            .create_draft(FormulaDefinition {
                id: "draft".into(),
                expression: source.into(),
            })
            .unwrap();
        assert_eq!(draft.state().diagnostics.len(), 1, "{source}");
        assert!(
            draft.state().diagnostics[0].message.starts_with("ifs()"),
            "{source}: {:?}",
            draft.state().diagnostics
        );
        assert_eq!(
            e.evaluate(&request(1)).unwrap().formulas[&"result".into()],
            Err(FormulaEvaluationError::NotReady),
            "{source}"
        );
    }
    for (source, expected) in [
        ("ifs(true, 1, 3)", Some(Value::Number(1.0))),
        ("ifs(false, 1, 3)", Some(Value::Number(3.0))),
        ("ifs(false, 1, empty())", None),
        ("ifs(empty(), 1, empty())", None),
        ("ifs(false, 1, true, 2, 3)", Some(Value::Number(2.0))),
        ("ifs(false, 1, false, 2, 3)", Some(Value::Number(3.0))),
        (
            "ifs(false, 1, false, 2, true, 3, 4)",
            Some(Value::Number(3.0)),
        ),
        (
            "ifs(false, 1, false, 2, false, 3, 4)",
            Some(Value::Number(4.0)),
        ),
        (
            "ifs(false, test(\"x\", \"[\"), true)",
            Some(Value::Boolean(true)),
        ),
        (
            "ifs(true, true, true, test(\"x\", \"[\"), test(\"x\", \"[\"))",
            Some(Value::Boolean(true)),
        ),
        (
            "ifs(true, true, test(\"x\", \"[\"), false, false)",
            Some(Value::Boolean(true)),
        ),
    ] {
        assert_eq!(value(source), expected, "{source}");
    }
    for (source, ty) in [
        ("ifs(false, 1, empty())", ValueType::Unknown),
        ("ifs(false, \"hit\", empty())", ValueType::Unknown),
        ("ifs(false, true, empty())", ValueType::Unknown),
        ("ifs(false, [1], empty())", ValueType::Unknown),
        ("ifs(false, 1, 3)", ValueType::Number),
        ("ifs(false, 1, false, 2, 3)", ValueType::Number),
        ("ifs(false, 1, false, 2, false, 3, 4)", ValueType::Number),
        (
            "ifs(false, 1, \"fallback\")",
            ValueType::Union(vec![ValueType::Number, ValueType::String]),
        ),
    ] {
        let e = engine(source);
        let draft = e
            .create_draft(FormulaDefinition {
                id: "draft".into(),
                expression: source.into(),
            })
            .unwrap();
        assert!(draft.state().diagnostics.is_empty(), "{source}");
        assert_eq!(draft.state().output_type, ty, "{source}");
        assert_eq!(output(&e, &request(1)).output_type, ty, "{source}");
    }
    for (source, ty) in [
        ("ifs(true, test(\"x\", \"[\"), empty())", ValueType::Unknown),
        ("ifs(false, true, test(\"x\", \"[\"))", ValueType::Boolean),
        (
            "ifs(false, true, test(\"x\", \"[\"), false, empty())",
            ValueType::Unknown,
        ),
    ] {
        let result = output(&engine(source), &request(1));
        assert_eq!(result.output_type, ty, "{source}");
        assert!(
            match &result.column {
                Column::Boolean(data) => !data.validity.is_valid(0),
                Column::Union(data) => !data.validity.is_valid(0),
                _ => false,
            },
            "{source}"
        );
        assert_eq!(result.errors.len(), 1, "{source}");
        assert_eq!(result.errors[0].row_index, 0, "{source}");
        assert_eq!(result.errors[0].origin_formula_id.0, "result", "{source}");
        assert!(
            matches!(&result.errors[0].error, RuntimeError::InvalidRegex { pattern, .. } if pattern == "["),
            "{source}"
        );
    }
    assert_eq!(value("empty()"), None);
    assert_eq!(value("empty(1)"), Some(Value::Boolean(false)));
    assert_eq!(
        value("slice([1, 2, 3], 1)"),
        Some(Value::List(vec![
            Some(Value::Number(2.0)),
            Some(Value::Number(3.0))
        ]))
    );
    assert_eq!(
        value("slice([1, 2, 3], 1, 2)"),
        Some(Value::List(vec![Some(Value::Number(2.0))]))
    );
}

#[test]
fn ifs_calls_keep_validation_strict_and_zero_row_batches_typed() {
    for source in ["ifs(true)", "ifs(1, 2)", "concat([1])", "sum()"] {
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
        ("ifs(false, 1, 3)", ValueType::Number),
        ("ifs(false, 1, empty())", ValueType::Unknown),
    ] {
        let result = output(&engine(source), &request(0));
        assert_eq!(result.output_type, ty, "{source}");
        assert!(result.errors.is_empty());
    }
}
