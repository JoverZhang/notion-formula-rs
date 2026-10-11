use std::collections::HashMap;

use formula_engine::{
    Column, EvaluateInput, FormulaDefinition, FormulaEngine, FormulaOutput, FormulaSchema,
    PropertyDefinition, RuntimeContext, Value,
};

pub fn formula(id: &str, expression: &str) -> PropertyDefinition {
    PropertyDefinition::Formula(FormulaDefinition {
        id: id.into(),
        expression: expression.into(),
    })
}

pub fn engine(expression: &str) -> FormulaEngine {
    FormulaEngine::new(FormulaSchema {
        properties: vec![formula("result", expression)],
    })
    .unwrap()
}

pub fn request(rows: usize) -> EvaluateInput {
    EvaluateInput {
        row_ids: (0..rows).map(|row| format!("row-{row}").into()).collect(),
        columns: HashMap::new(),
        runtime: RuntimeContext {
            now: 0,
            time_zone: "+00:00".into(),
        },
        formula_ids: vec!["result".into()],
    }
}

pub fn output(engine: &FormulaEngine, input: &EvaluateInput) -> FormulaOutput {
    let mut result = engine.evaluate(input).unwrap();
    result
        .formulas
        .remove(&"result".into())
        .unwrap()
        .unwrap_or_else(|error| panic!("{error:?}"))
}

pub fn value(expression: &str) -> Option<Value> {
    let result = output(&engine(expression), &request(1));
    assert!(
        result.errors.is_empty(),
        "{expression}: {:?}",
        result.errors
    );
    match result.column {
        Column::Number(data) => data
            .validity
            .is_valid(0)
            .then(|| Value::Number(data.values[0])),
        Column::String(data) => data
            .validity
            .is_valid(0)
            .then(|| Value::String(data.values[0].clone())),
        Column::Boolean(data) => data
            .validity
            .is_valid(0)
            .then(|| Value::Boolean(data.values[0])),
        Column::List(data) => data
            .validity
            .is_valid(0)
            .then(|| Value::List(data.values[0].clone())),
        Column::Union(data) => data.validity.is_valid(0).then(|| data.values[0].clone()),
        _ => panic!("unexpected date"),
    }
}
