use crate::completion::{
    CompletionData, CompletionItem, CompletionOutput, TextEdit, complete_with_context,
};
use crate::semantic::{Context, FunctionSig, ParamSig, Property, Ty};
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

fn pick_item<'a>(output: &'a CompletionOutput, label: &str) -> &'a CompletionItem {
    output
        .items
        .iter()
        .find(|item| item.label == label)
        .unwrap_or_else(|| panic!("missing completion item for label {label}"))
}

fn apply_text_edit(original: &str, edit: &TextEdit) -> (String, u32) {
    let start = edit.replace.start as usize;
    let end = edit.replace.end as usize;
    assert!(start <= end);
    assert!(end <= original.len());
    assert!(original.is_char_boundary(start));
    assert!(original.is_char_boundary(end));

    let mut updated = String::with_capacity(original.len() - (end - start) + edit.insert.len());
    updated.push_str(&original[..start]);
    updated.push_str(&edit.insert);
    updated.push_str(&original[end..]);

    assert!(edit.new_cursor as usize <= updated.len());
    (updated, edit.new_cursor)
}

#[test]
fn completion_at_document_start() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
                disabled_reason: None,
            },
        ],
        functions: vec![
            FunctionSig {
                name: "if".to_string(),
                params: vec![
                    ParamSig {
                        name: None,
                        ty: Ty::Boolean,
                        optional: false,
                    },
                    ParamSig {
                        name: None,
                        ty: Ty::Unknown,
                        optional: false,
                    },
                    ParamSig {
                        name: None,
                        ty: Ty::Unknown,
                        optional: false,
                    },
                ],
                ret: Ty::Unknown,
                detail: None,
            },
            FunctionSig {
                name: "sum".to_string(),
                params: vec![ParamSig {
                    name: None,
                    ty: Ty::Number,
                    optional: false,
                }],
                ret: Ty::Number,
                detail: None,
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
    let age_item = output
        .items
        .iter()
        .find(|item| item.label == r#"prop("Age")"#)
        .expect("expected prop(\"Age\") item");
    assert_eq!(
        age_item.data,
        Some(CompletionData::PropExpr {
            property_name: "Age".to_string()
        })
    );
    let if_item = output
        .items
        .iter()
        .find(|item| item.label == "if")
        .expect("expected if item");
    assert_eq!(
        if_item.data,
        Some(CompletionData::Function {
            name: "if".to_string()
        })
    );
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_at_document_start_without_context_has_no_functions() {
    let (output, cursor) = complete_fixture("$0", None);
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert!(!labels.contains(&"if"));
    assert!(!labels.contains(&"sum"));
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_at_document_start_without_properties_has_no_prop_variables() {
    let ctx = Context {
        properties: vec![],
        functions: vec![
            FunctionSig {
                name: "if".to_string(),
                params: vec![],
                ret: Ty::Unknown,
                detail: None,
            },
            FunctionSig {
                name: "sum".to_string(),
                params: vec![],
                ret: Ty::Number,
                detail: None,
            },
        ],
    };
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
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
                disabled_reason: None,
            },
        ],
        functions: vec![],
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
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
                disabled_reason: None,
            },
        ],
        functions: vec![],
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
    assert!(labels.is_empty());
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_inside_prop_string_with_context() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
                disabled_reason: None,
            },
        ],
        functions: vec![],
    };
    let (output, cursor) = complete_fixture(r#"prop("$0")"#, Some(ctx));
    let labels: Vec<&str> = output
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(labels, vec!["Title", "Age"]);
    let age_item = output
        .items
        .iter()
        .find(|item| item.label == "Age")
        .expect("expected Age item");
    assert_eq!(
        age_item.data,
        Some(CompletionData::PropertyName {
            name: "Age".to_string()
        })
    );

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
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
                disabled_reason: None,
            },
        ],
        functions: vec![],
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

#[test]
fn completion_disabled_property_marking() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
                disabled_reason: Some("cycle".to_string()),
            },
        ],
        functions: vec![],
    };
    let (output, cursor) = complete_fixture("$0", Some(ctx.clone()));
    let age_item = output
        .items
        .iter()
        .find(|item| item.label == r#"prop("Age")"#)
        .expect("expected prop(\"Age\") item");
    assert!(age_item.is_disabled);
    assert_eq!(age_item.disabled_reason.as_deref(), Some("cycle"));
    let title_item = output
        .items
        .iter()
        .find(|item| item.label == r#"prop("Title")"#)
        .expect("expected prop(\"Title\") item");
    assert!(!title_item.is_disabled);
    assert_eq!(title_item.disabled_reason, None);
    assert_replace_contains_cursor(output.replace, cursor);

    let (output, cursor) = complete_fixture(r#"prop("$0")"#, Some(ctx));
    let age_item = output
        .items
        .iter()
        .find(|item| item.label == "Age")
        .expect("expected Age item");
    assert!(age_item.is_disabled);
    assert_eq!(age_item.disabled_reason.as_deref(), Some("cycle"));
    let title_item = output
        .items
        .iter()
        .find(|item| item.label == "Title")
        .expect("expected Title item");
    assert!(!title_item.is_disabled);
    assert_eq!(title_item.disabled_reason, None);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_signature_help_active_param_first_arg() {
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig {
            name: "if".to_string(),
            params: vec![
                ParamSig {
                    name: Some("condition".to_string()),
                    ty: Ty::Boolean,
                    optional: false,
                },
                ParamSig {
                    name: Some("then".to_string()),
                    ty: Ty::Unknown,
                    optional: false,
                },
                ParamSig {
                    name: Some("else".to_string()),
                    ty: Ty::Unknown,
                    optional: false,
                },
            ],
            ret: Ty::Unknown,
            detail: None,
        }],
    };
    let (output, cursor) = complete_fixture("if($0", Some(ctx));
    let sig = output.signature_help.expect("expected signature help");
    assert_eq!(sig.active_param, 0);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_signature_help_active_param_second_arg() {
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig {
            name: "if".to_string(),
            params: vec![
                ParamSig {
                    name: Some("condition".to_string()),
                    ty: Ty::Boolean,
                    optional: false,
                },
                ParamSig {
                    name: Some("then".to_string()),
                    ty: Ty::Unknown,
                    optional: false,
                },
                ParamSig {
                    name: Some("else".to_string()),
                    ty: Ty::Unknown,
                    optional: false,
                },
            ],
            ret: Ty::Unknown,
            detail: None,
        }],
    };
    let (output, cursor) = complete_fixture("if(true, $0", Some(ctx));
    let sig = output.signature_help.expect("expected signature help");
    assert_eq!(sig.active_param, 1);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_signature_help_ignores_nested_commas() {
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig {
            name: "if".to_string(),
            params: vec![
                ParamSig {
                    name: Some("condition".to_string()),
                    ty: Ty::Boolean,
                    optional: false,
                },
                ParamSig {
                    name: Some("then".to_string()),
                    ty: Ty::Unknown,
                    optional: false,
                },
                ParamSig {
                    name: Some("else".to_string()),
                    ty: Ty::Unknown,
                    optional: false,
                },
            ],
            ret: Ty::Unknown,
            detail: None,
        }],
    };
    let (output, cursor) = complete_fixture("if(true, sum(1,2), $0", Some(ctx));
    let sig = output.signature_help.expect("expected signature help");
    assert_eq!(sig.active_param, 2);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_type_ranking_number_prefers_number_props() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
                disabled_reason: None,
            },
        ],
        functions: vec![
            FunctionSig {
                name: "if".to_string(),
                params: vec![
                    ParamSig {
                        name: None,
                        ty: Ty::Boolean,
                        optional: false,
                    },
                    ParamSig {
                        name: None,
                        ty: Ty::Unknown,
                        optional: false,
                    },
                    ParamSig {
                        name: None,
                        ty: Ty::Unknown,
                        optional: false,
                    },
                ],
                ret: Ty::Unknown,
                detail: None,
            },
            FunctionSig {
                name: "sum".to_string(),
                params: vec![ParamSig {
                    name: None,
                    ty: Ty::Number,
                    optional: false,
                }],
                ret: Ty::Number,
                detail: None,
            },
        ],
    };
    let (output, cursor) = complete_fixture("sum($0", Some(ctx));
    let age_idx = output
        .items
        .iter()
        .position(|item| item.label == r#"prop("Age")"#)
        .expect("expected prop(\"Age\") item");
    let title_idx = output
        .items
        .iter()
        .position(|item| item.label == r#"prop("Title")"#)
        .expect("expected prop(\"Title\") item");
    assert!(age_idx < title_idx);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_type_ranking_handles_nontrivial_property_names() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title (new)".to_string(),
                ty: Ty::Number,
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::String,
                disabled_reason: None,
            },
        ],
        functions: vec![FunctionSig {
            name: "sum".to_string(),
            params: vec![ParamSig {
                name: None,
                ty: Ty::Number,
                optional: false,
            }],
            ret: Ty::Number,
            detail: None,
        }],
    };
    let (output, cursor) = complete_fixture("sum($0", Some(ctx));
    let title_item = output
        .items
        .iter()
        .find(|item| item.label == r#"prop("Title (new)")"#)
        .expect("expected prop(\"Title (new)\") item");
    assert_eq!(
        title_item.data,
        Some(CompletionData::PropExpr {
            property_name: "Title (new)".to_string()
        })
    );
    let title_idx = output
        .items
        .iter()
        .position(|item| item.label == r#"prop("Title (new)")"#)
        .expect("expected prop(\"Title (new)\") item");
    let age_idx = output
        .items
        .iter()
        .position(|item| item.label == r#"prop("Age")"#)
        .expect("expected prop(\"Age\") item");
    assert!(title_idx < age_idx);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_type_ranking_boolean_prefers_literals() {
    let ctx = Context {
        properties: vec![
            Property {
                name: "Title".to_string(),
                ty: Ty::String,
                disabled_reason: None,
            },
            Property {
                name: "Age".to_string(),
                ty: Ty::Number,
                disabled_reason: None,
            },
        ],
        functions: vec![FunctionSig {
            name: "if".to_string(),
            params: vec![
                ParamSig {
                    name: None,
                    ty: Ty::Boolean,
                    optional: false,
                },
                ParamSig {
                    name: None,
                    ty: Ty::Unknown,
                    optional: false,
                },
                ParamSig {
                    name: None,
                    ty: Ty::Unknown,
                    optional: false,
                },
            ],
            ret: Ty::Unknown,
            detail: None,
        }],
    };
    let (output, cursor) = complete_fixture("if($0", Some(ctx));
    let true_idx = output
        .items
        .iter()
        .position(|item| item.label == "true")
        .expect("expected true item");
    let title_idx = output
        .items
        .iter()
        .position(|item| item.label == r#"prop("Title")"#)
        .expect("expected prop(\"Title\") item");
    assert!(true_idx < title_idx);
    assert_replace_contains_cursor(output.replace, cursor);
}

#[test]
fn completion_apply_function_inserts_lparen_and_moves_cursor() {
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig {
            name: "if".to_string(),
            params: vec![],
            ret: Ty::Unknown,
            detail: None,
        }],
    };
    let (output, _cursor) = complete_fixture("$0", Some(ctx));
    let item = pick_item(&output, "if");
    let edit = item.text_edit.as_ref().expect("expected text edit");
    let (updated, new_cursor) = apply_text_edit("", edit);
    assert_eq!(updated, "if(");
    assert_eq!(new_cursor, 3);
}

#[test]
fn completion_apply_function_replaces_prefix() {
    let ctx = Context {
        properties: vec![],
        functions: vec![FunctionSig {
            name: "sum".to_string(),
            params: vec![],
            ret: Ty::Number,
            detail: None,
        }],
    };
    let (output, _cursor) = complete_fixture("su$0", Some(ctx));
    let item = pick_item(&output, "sum");
    let edit = item.text_edit.as_ref().expect("expected text edit");
    let (updated, new_cursor) = apply_text_edit("su", edit);
    assert_eq!(updated, "sum(");
    assert_eq!(new_cursor, 4);
}

#[test]
fn completion_apply_prop_expr_places_cursor_at_end() {
    let ctx = Context {
        properties: vec![Property {
            name: "Age".to_string(),
            ty: Ty::Number,
            disabled_reason: None,
        }],
        functions: vec![],
    };
    let (output, _cursor) = complete_fixture("$0", Some(ctx));
    let item = pick_item(&output, r#"prop("Age")"#);
    let edit = item.text_edit.as_ref().expect("expected text edit");
    let expected = r#"prop("Age")"#;
    let (updated, new_cursor) = apply_text_edit("", edit);
    assert_eq!(updated, expected);
    assert_eq!(new_cursor, expected.len() as u32);
}

#[test]
fn completion_apply_property_name_inside_prop_string() {
    let ctx = Context {
        properties: vec![Property {
            name: "Age".to_string(),
            ty: Ty::Number,
            disabled_reason: None,
        }],
        functions: vec![],
    };
    let (output, _cursor) = complete_fixture(r#"prop("$0")"#, Some(ctx));
    let item = pick_item(&output, "Age");
    let edit = item.text_edit.as_ref().expect("expected text edit");
    let (updated, new_cursor) = apply_text_edit(r#"prop("")"#, edit);
    let expected = r#"prop("Age")"#;
    let expected_cursor = expected.find("Age").unwrap() + "Age".len();
    assert_eq!(updated, expected);
    assert_eq!(new_cursor as usize, expected_cursor);
}

#[test]
fn completion_disabled_item_has_no_text_edit() {
    let ctx = Context {
        properties: vec![Property {
            name: "Age".to_string(),
            ty: Ty::Number,
            disabled_reason: Some("cycle".to_string()),
        }],
        functions: vec![],
    };
    let (output, _cursor) = complete_fixture("$0", Some(ctx));
    let item = pick_item(&output, r#"prop("Age")"#);
    assert!(item.is_disabled);
    assert!(item.text_edit.is_none());
}
