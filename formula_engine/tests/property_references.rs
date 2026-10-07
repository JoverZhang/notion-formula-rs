//! Reference spans let an editor bind chips without a second formula parser.
//! Cover decoded IDs, missing fields, comments/strings, unsupported member
//! access, incomplete calls, UTF-8 boundaries, and detached state snapshots.
use formula_engine::{ExpressionUpdate, FormulaDefinition, FormulaEngine, FormulaSchema};

#[test]
fn draft_exposes_complete_literal_references_in_source_order() {
    let engine = FormulaEngine::new(FormulaSchema { properties: vec![] }).unwrap();
    let expression = r#"["中🙂", prop("字段"), prop("missing"), prop("q\"id"), "prop(\"string\")", [].prop("foreign"), /* prop("comment") */ prop("字段")]"#;
    let draft = engine
        .create_draft(FormulaDefinition {
            id: "candidate".into(),
            expression: expression.into(),
        })
        .unwrap();
    let references = &draft.state().property_references;
    assert_eq!(references.len(), 4);
    for (reference, (id, call, literal)) in references.iter().zip([
        ("字段", r#"prop("字段")"#, r#""字段""#),
        ("missing", r#"prop("missing")"#, r#""missing""#),
        ("q\"id", r#"prop("q\"id")"#, r#""q\"id""#),
        ("字段", r#"prop("字段")"#, r#""字段""#),
    ]) {
        assert_eq!(reference.property_id.0, id);
        assert_eq!(
            &expression[reference.span.start as usize..reference.span.end as usize],
            call
        );
        assert_eq!(
            &expression[reference.id_span.start as usize..reference.id_span.end as usize],
            literal
        );
    }
    assert!(
        references
            .windows(2)
            .all(|pair| pair[0].span.end <= pair[1].span.start)
    );
}

#[test]
fn references_follow_edits_without_mutating_old_snapshots_or_hiding_incomplete_calls() {
    let engine = FormulaEngine::new(FormulaSchema { properties: vec![] }).unwrap();
    let mut draft = engine
        .create_draft(FormulaDefinition {
            id: "candidate".into(),
            expression: r#"prop( /* keep */ "old" )"#.into(),
        })
        .unwrap();
    let before = draft.state().clone();
    assert_eq!(before.property_references[0].property_id.0, "old");
    for source in [
        r#"prop("unfinished""#,
        "prop(name)",
        "prop()",
        r#"prop("a", "b")"#,
        r#"prop("a",)"#,
        r#"prop("a" junk)"#,
        r#"prop("a" /*)*/"#,
        r#"prop("a"]"#,
        r#"prop("bad\q")"#,
    ] {
        let update = draft
            .update_expression(ExpressionUpdate::Replace(source.into()))
            .unwrap();
        assert!(update.state.property_references.is_empty(), "{source}");
    }
    let update = draft
        .update_expression(ExpressionUpdate::Replace(
            r#""🙂" + prop("new") + ("#.into(),
        ))
        .unwrap();
    assert_eq!(update.state.property_references.len(), 1);
    assert_eq!(update.state.property_references[0].property_id.0, "new");
    assert!(!update.state.diagnostics.is_empty());
    assert_eq!(before.property_references[0].property_id.0, "old");
}
