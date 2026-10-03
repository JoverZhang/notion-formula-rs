use formula_engine::{FormulaDefinition, FormulaEngine, FormulaSchema};

fn main() {
    let mut engine = FormulaEngine::new(FormulaSchema { properties: vec![] }).unwrap();
    let draft = engine
        .create_draft(FormulaDefinition {
            id: "draft".into(),
            expression: "1".into(),
        })
        .unwrap();
    engine.remove(&"input".into());
    assert_eq!(draft.state().definition.expression, "1");
}
