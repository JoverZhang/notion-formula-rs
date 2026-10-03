use formula_engine::{FormulaDefinition, FormulaEngine, FormulaSchema, PropertyDefinition};

pub fn edit_then_save() {
    let mut engine = FormulaEngine::new(FormulaSchema { properties: vec![] }).unwrap();
    let first = engine
        .create_draft(FormulaDefinition {
            id: "first".into(),
            expression: "1".into(),
        })
        .unwrap();
    let second = engine
        .create_draft(FormulaDefinition {
            id: "second".into(),
            expression: "2".into(),
        })
        .unwrap();
    assert_eq!(first.state().definition.expression, "1");
    assert_eq!(second.state().definition.expression, "2");
    let first = first.into_definition();
    let second = second.into_definition();
    engine.upsert(PropertyDefinition::Formula(first)).unwrap();
    engine.upsert(PropertyDefinition::Formula(second)).unwrap();
}
