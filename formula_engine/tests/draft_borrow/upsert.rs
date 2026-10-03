use formula_engine::{
    FormulaDefinition, FormulaEngine, FormulaSchema, PropertyDefinition, ValueType,
};

fn main() {
    let mut engine = FormulaEngine::new(FormulaSchema { properties: vec![] }).unwrap();
    let draft = engine
        .create_draft(FormulaDefinition {
            id: "draft".into(),
            expression: "1".into(),
        })
        .unwrap();
    engine
        .upsert(PropertyDefinition::Input {
            id: "input".into(),
            ty: ValueType::Number,
        })
        .unwrap();
    assert_eq!(draft.state().definition.expression, "1");
}
