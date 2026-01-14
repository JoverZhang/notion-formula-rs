use crate::completion::{CompletionOutput, complete_with_context};
use crate::semantic::{Context, Property, Ty};
use crate::token::Span;

fn complete_fixture(input: &str, ctx: Option<Context>) -> (CompletionOutput, u32) {
    let cursor = input.find("$0").expect("fixture must contain $0 marker");
    let text = input.to_string();
    let replaced = text.replace("$0", "");
    assert!(
        replaced.len() + 2 == text.len(),
        "fixture must contain exactly one $0 marker"
    );
    let output = complete_with_context(&replaced, cursor, ctx.as_ref());
    (output, cursor as u32)
}

fn assert_replace_contains_cursor(replace: Span, cursor: u32) {
    assert!(
        replace.start <= cursor && cursor <= replace.end,
        "replace span must contain cursor: {:?} vs {}",
        replace,
        cursor
    );
}

#[test]
fn completion_at_document_start() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
            },
        ],
    };
    let (output, cursor) = complete_fixture("$0", Some(ctx));
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(labels[0], "prop(\"Title\")");
    assert_eq!(labels[1], "prop(\"Age\")");
    assert!(labels.contains(&"if"));
    assert!(labels.contains(&"sum"));
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_at_document_start_without_properties_has_no_prop_variables() {
    let ctx = Context { properties: vec![] };
    let (output, cursor) = complete_fixture("$0", Some(ctx));
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert!(!labels.iter().any(|label| label.starts_with("prop(\"")));
    assert!(labels.contains(&"if"));
    assert!(labels.contains(&"sum"));
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_identifier_suppresses_expr_start() {
    let (output, cursor) = complete_fixture("abc$0", None);
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert!(!labels.iter().any(|label| label.starts_with("prop(\"")));
    assert!(!labels.contains(&"if"));
    assert!(!labels.contains(&"sum"));
    assert!(!labels.contains(&"true"));
    assert!(!labels.contains(&"false"));
    assert!(!labels.contains(&"("));
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_prop_ident_with_context() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
            },
        ],
    };
    let (output, cursor) = complete_fixture("prop$0", Some(ctx));
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(labels, vec![r#"prop("Title")"#, r#"prop("Age")"#]);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_prop_ident_without_context() {
    let (output, cursor) = complete_fixture("prop$0", None);
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(labels, vec!["("]);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_prop_lparen_with_context() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
            },
        ],
    };
    let (output, cursor) = complete_fixture("prop($0", Some(ctx));
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(labels, vec!["prop(\"Title\")", "prop(\"Age\")"]);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_prop_lparen_without_context() {
    let (output, cursor) = complete_fixture("prop($0", None);
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(labels, vec!["\""]);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_inside_prop_string_with_context() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
            },
        ],
    };
    let (output, cursor) = complete_fixture(r#"prop("$0")"#, Some(ctx));
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(labels, vec!["Title", "Age"]);

    let text = r#"prop("")"#;
    let quote_start = text.find('"').expect("expected opening quote");
    let quote_end = text[quote_start + 1..]
        .find('"')
        .map(|idx| idx + quote_start + 1)
        .expect("expected closing quote");
    let expected = Span {
        start: (quote_start + 1) as u32,
        end: quote_end as u32,
    };
    assert_eq!(output.replace, expected);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_complete_atom_suppresses_expr_start() {
    let (output, cursor) = complete_fixture("prop(\"Title\")$0", None);
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert!(!labels.iter().any(|label| label.starts_with("prop(\"")));
    assert!(!labels.contains(&"if"));
    assert!(!labels.contains(&"sum"));
    assert!(!labels.contains(&"true"));
    assert!(!labels.contains(&"false"));
    assert!(!labels.contains(&"("));
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_prefix_prop_identifier_with_context() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
            },
        ],
    };
    let (output, cursor) = complete_fixture("pro$0", Some(ctx));
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(labels[0], r#"prop("Title")"#);
    assert_eq!(labels[1], r#"prop("Age")"#);
    assert_replace_contains_cursor(output.replace, cursor);
}
