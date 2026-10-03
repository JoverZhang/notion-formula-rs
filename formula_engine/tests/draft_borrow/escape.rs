use formula_engine::{FormulaDefinition, FormulaDraft, FormulaEngine, FormulaSchema};

pub fn escape_local_engine<'engine>() -> FormulaDraft<'engine> {
    let engine = FormulaEngine::new(FormulaSchema { properties: vec![] }).unwrap();
    engine
        .create_draft(FormulaDefinition {
            id: "draft".into(),
            expression: "1".into(),
        })
        .unwrap()
}
