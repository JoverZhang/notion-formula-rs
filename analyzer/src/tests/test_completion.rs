use crate::completion::{complete_with_context, CompletionOutput};
use crate::semantic::{Context, Property, Ty};
use crate::token::Span;

fn complete_fixture(input: &str, ctx: Option<Context>) -> (CompletionOutput, u32) {
    let cursor = input
        .find("$0")
        .expect("fixture must contain $0 marker");
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
    let (output, cursor) = complete_fixture("$0", None);
    let labels: Vec<&str> = output.items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, vec!["prop(\"", "if(", "sum(", "true", "false", "("]);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_identifier_suppresses_expr_start() {
    let (output, cursor) = complete_fixture("abc$0", None);
    let labels: Vec<&str> = output.items.iter().map(|item| item.label.as_str()).collect();
    assert!(!labels.contains(&"prop(\""));
    assert!(!labels.contains(&"if("));
    assert!(!labels.contains(&"sum("));
    assert!(!labels.contains(&"true"));
    assert!(!labels.contains(&"false"));
    assert!(!labels.contains(&"("));
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_prop_ident() {
    let (output, cursor) = complete_fixture("prop$0", None);
    let labels: Vec<&str> = output.items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, vec!["("]);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_after_prop_lparen() {
    let (output, cursor) = complete_fixture("prop($0", None);
    let labels: Vec<&str> = output.items.iter().map(|item| item.label.as_str()).collect();
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
    let (output, cursor) = complete_fixture("prop(\"$0\")", Some(ctx));
    let labels: Vec<&str> = output.items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, vec!["Title", "Age"]);

    let text = "prop(\"\")";
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
    let labels: Vec<&str> = output.items.iter().map(|item| item.label.as_str()).collect();
    assert!(!labels.contains(&"prop(\""));
    assert!(!labels.contains(&"if("));
    assert!(!labels.contains(&"sum("));
    assert!(!labels.contains(&"true"));
    assert!(!labels.contains(&"false"));
    assert!(!labels.contains(&"("));
    assert_replace_contains_cursor(output.replace, cursor);
}
