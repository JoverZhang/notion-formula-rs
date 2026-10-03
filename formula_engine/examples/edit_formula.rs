//! Run with `cargo run --locked -p formula_engine --example edit_formula`.

use formula_engine::{
    Column, ColumnData, EvaluateInput, ExpressionUpdate, FormulaDefinition, FormulaEngine,
    FormulaSchema, NullBuffer, PropertyDefinition, RuntimeContext, ValueType,
};

fn main() {
    let mut engine = FormulaEngine::new(FormulaSchema {
        properties: vec![
            PropertyDefinition::Input {
                id: "Price".into(),
                ty: ValueType::Number,
            },
            PropertyDefinition::Formula(FormulaDefinition {
                id: "Total".into(),
                expression: "prop(\"Price\") * 2".into(),
            }),
        ],
    })
    .expect("valid definitions");

    let input = EvaluateInput {
        row_ids: vec!["first".into(), "second".into()],
        columns: [(
            "Price".into(),
            Column::Number(ColumnData {
                values: vec![10.0, 20.0],
                validity: NullBuffer::new_valid(2),
            }),
        )]
        .into(),
        runtime: RuntimeContext {
            now: 0,
            time_zone: "+08:00".into(),
        },
        formula_ids: vec!["Total".into()],
    };

    print_total("Before editing", &engine, &input);
    let mut draft = engine
        .create_draft(FormulaDefinition {
            id: "Total".into(),
            expression: "prop(\"Price\") * 2".into(),
        })
        .expect("nonempty ID");

    draft
        .update_expression(ExpressionUpdate::Replace("prop(\"Price\") * 3".into()))
        .expect("valid replacement");
    println!(
        "Draft version {}: {} ({:?}, {} diagnostics)",
        draft.state().version.0,
        draft.state().definition.expression,
        draft.state().output_type,
        draft.state().diagnostics.len(),
    );
    print_total("Engine while draft is open", &engine, &input);

    let definition = draft.into_definition();
    engine
        .upsert(PropertyDefinition::Formula(definition))
        .expect("nonempty ID");
    print_total("After saving", &engine, &input);
}

fn print_total(label: &str, engine: &FormulaEngine, input: &EvaluateInput) {
    let result = engine.evaluate(input).expect("valid input");
    let output = result.formulas[&"Total".into()]
        .as_ref()
        .expect("ready formula");
    assert!(output.errors.is_empty());
    let Column::Number(column) = &output.column else {
        panic!("expected a Number column");
    };
    println!("{label}: {:?}", column.values);
}
