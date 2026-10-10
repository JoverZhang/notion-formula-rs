use formula_engine::{
    EvaluateInputError, FormulaDefinition, FormulaEngine, FormulaSchema, PropertyDefinition,
    PropertyId, ValueType,
};

fn formula(id: &str, expression: &str) -> PropertyDefinition {
    PropertyDefinition::Formula(FormulaDefinition {
        id: id.into(),
        expression: expression.into(),
    })
}

fn input(id: &str) -> PropertyDefinition {
    PropertyDefinition::Input {
        id: id.into(),
        ty: ValueType::Number,
    }
}

fn engine(properties: Vec<PropertyDefinition>) -> FormulaEngine {
    FormulaEngine::new(FormulaSchema { properties }).unwrap()
}

fn ids(values: &[&str]) -> Vec<PropertyId> {
    values.iter().map(|id| (*id).into()).collect()
}

#[test]
fn static_input_closure_follows_transitive_references_and_schema_changes() {
    let mut engine = engine(vec![
        input("Price"),
        input("Quantity"),
        input("unused"),
        formula("Subtotal", r#"prop("Price") * prop("Quantity")"#),
        formula("Total", r#"prop("Subtotal") + prop("Price")"#),
        formula("constant", "1"),
        formula("branches", r#"if(true, prop("Price"), prop("Quantity"))"#),
    ]);
    assert_eq!(
        engine.required_inputs(&ids(&["Total", "Subtotal", "branches"])),
        Ok(ids(&["Price", "Quantity"]))
    );
    assert_eq!(engine.required_inputs(&ids(&["constant"])), Ok(vec![]));

    engine
        .upsert(formula("Subtotal", r#"prop("unused")"#))
        .unwrap();
    assert_eq!(
        engine.required_inputs(&ids(&["Total"])),
        Ok(ids(&["Price", "unused"]))
    );
    engine.upsert(input("Subtotal")).unwrap();
    assert_eq!(
        engine.required_inputs(&ids(&["Total"])),
        Ok(ids(&["Price", "Subtotal"]))
    );
    engine.remove(&"Subtotal".into()).unwrap();
    assert_eq!(
        engine.required_inputs(&ids(&["Total"])),
        Ok(ids(&["Price"]))
    );
}

#[test]
fn not_ready_formulas_and_cycles_still_expose_known_inputs() {
    let engine = engine(vec![
        input("a"),
        input("b"),
        formula("type_error", r#"prop("a") + true"#),
        formula("syntax_error", r#"prop("b") +"#),
        formula("missing", r#"prop("absent") + prop("a")"#),
        formula("cycle1", r#"prop("cycle2") + prop("a")"#),
        formula("cycle2", r#"prop("cycle1") + prop("b")"#),
        formula("self", r#"prop("self") + prop("a")"#),
    ]);
    assert_eq!(
        engine.required_inputs(&ids(&["type_error", "syntax_error", "missing"])),
        Ok(ids(&["a", "b"]))
    );
    assert_eq!(
        engine.required_inputs(&ids(&["cycle1"])),
        Ok(ids(&["a", "b"]))
    );
    assert_eq!(engine.required_inputs(&ids(&["self"])), Ok(ids(&["a"])));
}

#[test]
fn formula_selection_uses_existing_validation_errors_in_request_order() {
    let engine = engine(vec![input("input"), formula("valid", "1")]);
    assert_eq!(
        engine.required_inputs(&[]),
        Err(EvaluateInputError::EmptyFormulaIds)
    );
    for id in ["", "absent", "input"] {
        assert_eq!(
            engine.required_inputs(&ids(&[id])),
            Err(EvaluateInputError::InvalidFormulaId { id: id.into() })
        );
    }
    assert_eq!(
        engine.required_inputs(&ids(&["valid", "valid", "absent"])),
        Err(EvaluateInputError::DuplicateFormulaId { id: "valid".into() })
    );
    assert_eq!(
        engine.required_inputs(&ids(&["absent", "valid", "valid"])),
        Err(EvaluateInputError::InvalidFormulaId {
            id: "absent".into()
        })
    );
}

#[test]
fn long_dependency_closure_uses_a_bounded_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            const COUNT: usize = 2_500;
            let mut properties = vec![input("leaf")];
            for index in 0..COUNT {
                let expression = if index + 1 == COUNT {
                    r#"prop("leaf")"#.into()
                } else {
                    format!("prop(\"f{:05}\")", index + 1)
                };
                properties.push(formula(&format!("f{index:05}"), &expression));
            }
            let engine = engine(properties);
            assert_eq!(
                engine.required_inputs(&ids(&["f00000"])),
                Ok(ids(&["leaf"]))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
