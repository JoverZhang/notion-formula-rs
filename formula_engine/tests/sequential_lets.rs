#[path = "support/builtin_eval.rs"]
mod builtin_eval;

use builtin_eval::{engine, output, request, value};
use formula_engine::{FormulaDefinition, Value, ValueType};

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
fn lets_keeps_validation_strict_and_zero_row_batches_typed() {
    for source in ["lets(1, 2, 3)", "lets(x, 1)"] {
        let e = engine(source);
        let draft = e
            .create_draft(FormulaDefinition {
                id: "draft".into(),
                expression: source.into(),
            })
            .unwrap();
        assert!(!draft.state().diagnostics.is_empty(), "{source}");
    }
    for (source, ty) in [("lets(x, 1, x)", ValueType::Number)] {
        let result = output(&engine(source), &request(0));
        assert_eq!(result.output_type, ty, "{source}");
        assert!(result.errors.is_empty());
    }
}
