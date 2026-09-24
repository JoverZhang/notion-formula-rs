use std::fmt::Write;

use formula_engine::{
    EngineChangeError, FormulaDefinition, FormulaEngine, FormulaEngineChangeResult,
    FormulaEngineInitError, FormulaEngineState, FormulaSchema, FormulaStatus, PropertyDefinition,
    PropertyId, PropertyState, ValueType,
};

fn formula(id: &str, expression: &str) -> PropertyDefinition {
    PropertyDefinition::Formula(FormulaDefinition {
        id: id.into(),
        expression: expression.into(),
    })
}

fn input(id: &str, ty: ValueType) -> PropertyDefinition {
    PropertyDefinition::Input { id: id.into(), ty }
}

fn make_engine(properties: Vec<PropertyDefinition>) -> FormulaEngine {
    FormulaEngine::new(FormulaSchema { properties }).expect("valid schema")
}

fn change_ids(result: FormulaEngineChangeResult) -> Vec<String> {
    let mut ids: Vec<_> = result
        .affected_formulas
        .into_iter()
        .map(|id| id.0)
        .collect();
    ids.sort();
    ids
}

fn status(engine: &FormulaEngine, id: &str) -> String {
    match engine.property(&id.into()) {
        Some(PropertyState::Input { ty, .. }) => format!("Input({ty:?})"),
        Some(PropertyState::Formula(state)) => match state.status {
            FormulaStatus::Ready { output_type } => format!("Ready({output_type:?})"),
            FormulaStatus::NotReady => "NotReady".into(),
        },
        None => "Missing".into(),
    }
}

fn engine_state(engine: &FormulaEngine) -> String {
    match engine.state() {
        FormulaEngineState::AllReady => "AllReady".into(),
        FormulaEngineState::NotAllReady { cycle_path } => {
            let ids: Vec<_> = cycle_path.iter().map(|id| id.0.as_str()).collect();
            format!("NotAllReady({ids:?})")
        }
    }
}

fn record(out: &mut String, label: &str, engine: &FormulaEngine, ids: &[&str]) {
    write!(out, "{label}: {}", engine_state(engine)).unwrap();
    for id in ids {
        write!(out, " {id}={}", status(engine, id)).unwrap();
    }
    writeln!(out).unwrap();
}

#[test]
fn definitions_scenario_transcript() {
    let declarations = vec![
        formula("copied", r#"prop("nested")"#),
        formula("first", r#"first(prop("refined"))"#),
        formula("refined", r#"concat(prop("empty"), [1, 2])"#),
        formula("empty", "[]"),
        formula("length", r#"length(prop("empty"))"#),
        formula("dynamic_copy", r#"prop("dynamic")"#),
        formula("quoted", r#"prop("a\"b")"#),
        input(
            "nested",
            ValueType::Union(vec![
                ValueType::List(Box::new(ValueType::Unknown)),
                ValueType::Number,
            ]),
        ),
        input("dynamic", ValueType::Unknown),
        input("a\"b", ValueType::String),
    ];
    let forward = make_engine(declarations.clone());
    let backward = make_engine(declarations.into_iter().rev().collect());
    let type_ids = [
        "copied",
        "dynamic_copy",
        "empty",
        "first",
        "length",
        "quoted",
        "refined",
    ];
    for id in type_ids {
        assert_eq!(status(&forward, id), status(&backward, id), "{id}");
    }
    assert_eq!(engine_state(&forward), engine_state(&backward));
    assert_eq!(forward.properties(), backward.properties());

    let mut out = String::new();
    record(&mut out, "order and types", &forward, &type_ids);

    let mut engine = make_engine(vec![
        formula("A", r#"abs(prop("B"))"#),
        formula("C", r#"prop("A") * 2"#),
        formula("U", "9"),
    ]);
    let chain = ["A", "B", "C", "U"];
    record(&mut out, "missing B", &engine, &chain);
    let old_a = engine.property(&"A".into()).unwrap();
    let old_properties = engine.properties();

    let affected = change_ids(engine.upsert(formula("B", "3")).unwrap());
    writeln!(out, "add B affected={affected:?}").unwrap();
    record(&mut out, "after add", &engine, &chain);
    assert!(matches!(
        old_a,
        PropertyState::Formula(state) if matches!(state.status, FormulaStatus::NotReady)
    ));
    assert_eq!(old_properties.len(), 3);
    assert!(old_properties.iter().all(|property| {
        !matches!(property, PropertyState::Formula(state) if state.definition.id.0 == "B")
    }));

    let affected = change_ids(engine.upsert(formula("B", "3")).unwrap());
    writeln!(out, "same B affected={affected:?}").unwrap();
    let affected = change_ids(engine.upsert(formula("B", "1 +")).unwrap());
    writeln!(out, "syntax error affected={affected:?}").unwrap();
    record(&mut out, "after syntax error", &engine, &chain);
    let affected = change_ids(engine.upsert(formula("B", r#"abs("x")"#)).unwrap());
    writeln!(out, "type error affected={affected:?}").unwrap();
    record(&mut out, "after type error", &engine, &chain);
    let affected = change_ids(engine.upsert(formula("B", "4")).unwrap());
    writeln!(out, "repair B affected={affected:?}").unwrap();
    record(&mut out, "after repair", &engine, &chain);

    let affected = change_ids(engine.upsert(input("B", ValueType::Number)).unwrap());
    writeln!(out, "formula to input affected={affected:?}").unwrap();
    record(&mut out, "after input", &engine, &chain);
    let affected = change_ids(engine.upsert(input("B", ValueType::String)).unwrap());
    writeln!(out, "input type change affected={affected:?}").unwrap();
    record(&mut out, "after string input", &engine, &chain);
    let affected = change_ids(engine.upsert(input("B", ValueType::Number)).unwrap());
    writeln!(out, "input repair affected={affected:?}").unwrap();
    record(&mut out, "after input repair", &engine, &chain);
    let affected = change_ids(engine.upsert(formula("B", "5")).unwrap());
    writeln!(out, "input to formula affected={affected:?}").unwrap();
    record(&mut out, "after formula", &engine, &chain);

    let before_invalid = engine.properties();
    assert_eq!(
        engine.upsert(input("", ValueType::Unknown)).unwrap_err(),
        EngineChangeError::EmptyId
    );
    assert_eq!(engine.properties(), before_invalid);
    let affected = change_ids(engine.remove(&"B".into()).unwrap());
    writeln!(out, "remove B affected={affected:?}").unwrap();
    record(&mut out, "after remove", &engine, &chain);
    assert!(engine.remove(&"B".into()).is_none());

    assert_eq!(out, include_str!("definitions.snap"));
}

#[test]
fn cycles_ids_and_snapshots() {
    let mut engine = make_engine(vec![
        formula("A", r#"prop("B")"#),
        formula("B", r#"prop("C")"#),
        formula("C", r#"prop("A")"#),
        formula("D", r#"prop("A")"#),
        formula("safe", "1"),
    ]);
    assert_eq!(status(&engine, "safe"), "Ready(Number)");
    assert_eq!(
        status(&make_engine(vec![formula("blank", "")]), "blank"),
        "NotReady"
    );
    assert_eq!(status(&engine, "D"), "NotReady");
    assert_cycle(&engine, &[("A", "B"), ("B", "C"), ("C", "A")]);

    let snapshot = engine.property(&"A".into()).unwrap();
    assert_eq!(
        change_ids(engine.upsert(formula("C", "1")).unwrap()),
        ["A", "B", "C", "D"]
    );
    assert_eq!(engine_state(&engine), "AllReady");
    assert!(matches!(
        snapshot,
        PropertyState::Formula(state) if matches!(state.status, FormulaStatus::NotReady)
    ));

    assert_eq!(
        change_ids(engine.upsert(formula("C", r#"prop("C")"#)).unwrap()),
        ["A", "B", "C", "D"]
    );
    assert_cycle(&engine, &[("C", "C")]);
    engine.upsert(formula("C", "1")).unwrap();
    engine
        .upsert(formula("unready", r#"prop("missing")"#))
        .unwrap();
    assert_eq!(engine_state(&engine), "NotAllReady([])");
    assert_eq!(status(&engine, "safe"), "Ready(Number)");
    assert_eq!(status(&engine, "unready"), "NotReady");

    let mut exact_ids = make_engine(vec![
        input("Å", ValueType::Number),
        input("Å", ValueType::Number),
        input("name", ValueType::Number),
        input("Name", ValueType::String),
        formula(
            "sum",
            r#"abs(prop("Å")) + abs(prop("Å")) + abs(prop("name"))"#,
        ),
        formula("capitalized", r#"prop("Name")"#),
    ]);
    assert_eq!(engine_state(&exact_ids), "AllReady");
    assert_eq!(status(&exact_ids, "sum"), "Ready(Number)");
    assert_eq!(status(&exact_ids, "capitalized"), "Ready(String)");
    assert_eq!(exact_ids.properties().len(), 6);
    assert!(exact_ids.property(&PropertyId::from("NAME")).is_none());
    assert_eq!(
        change_ids(exact_ids.upsert(input("Å", ValueType::String)).unwrap()),
        ["sum"]
    );
    assert_eq!(status(&exact_ids, "sum"), "NotReady");
    assert_eq!(status(&exact_ids, "Å"), "Input(Number)");
    assert_eq!(status(&exact_ids, "capitalized"), "Ready(String)");

    assert_eq!(
        FormulaEngine::new(FormulaSchema {
            properties: vec![input("", ValueType::Number)]
        })
        .err()
        .unwrap(),
        FormulaEngineInitError::EmptyId
    );
    assert_eq!(
        FormulaEngine::new(FormulaSchema {
            properties: vec![input("same", ValueType::Number), formula("same", "1")],
        })
        .err()
        .unwrap(),
        FormulaEngineInitError::DuplicateId("same".into())
    );
}

#[test]
fn long_chain_cycle_and_recovery_fit_worker_stack() {
    const COUNT: usize = 2_500;
    std::thread::Builder::new()
        .name("formula-engine-long-chain".into())
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let properties = (0..COUNT)
                .rev()
                .map(|index| {
                    let id = format!("f{index:05}");
                    let expression = if index + 1 == COUNT {
                        "1".to_owned()
                    } else {
                        format!("prop(\"f{:05}\")", index + 1)
                    };
                    formula(&id, &expression)
                })
                .collect();
            let mut engine = make_engine(properties);
            assert_eq!(engine_state(&engine), "AllReady");
            assert_eq!(status(&engine, "f00000"), "Ready(Number)");
            assert_eq!(status(&engine, "f02499"), "Ready(Number)");

            let affected = engine
                .upsert(formula("f02499", r#"prop("f00000")"#))
                .unwrap()
                .affected_formulas;
            assert_eq!(affected.len(), COUNT);
            {
                let FormulaEngineState::NotAllReady { cycle_path } = engine.state() else {
                    panic!("expected a cycle");
                };
                assert_eq!(cycle_path.len(), COUNT + 1);
                assert_eq!(cycle_path.first(), cycle_path.last());
                for pair in cycle_path.windows(2) {
                    let from: usize = pair[0].0.strip_prefix('f').unwrap().parse().unwrap();
                    let to: usize = pair[1].0.strip_prefix('f').unwrap().parse().unwrap();
                    assert_eq!(to, (from + 1) % COUNT);
                }
            }
            assert_eq!(status(&engine, "f00000"), "NotReady");

            let affected = engine.upsert(formula("f02499", "1")).unwrap();
            assert_eq!(affected.affected_formulas.len(), COUNT);
            assert_eq!(engine_state(&engine), "AllReady");
            assert_eq!(status(&engine, "f00000"), "Ready(Number)");
        })
        .unwrap()
        .join()
        .unwrap();
}

fn assert_cycle(engine: &FormulaEngine, direct_edges: &[(&str, &str)]) {
    let FormulaEngineState::NotAllReady { cycle_path } = engine.state() else {
        panic!("expected cycle");
    };
    assert!(cycle_path.len() >= 2);
    assert_eq!(cycle_path.first(), cycle_path.last());
    for pair in cycle_path.windows(2) {
        assert!(
            direct_edges
                .iter()
                .any(|(from, to)| pair[0].0 == *from && pair[1].0 == *to),
            "invalid edge: {pair:?}"
        );
    }
}
