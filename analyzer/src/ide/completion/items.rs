use super::{CompletionData, CompletionItem, CompletionKind};
use crate::semantic;

pub(super) fn expr_start_items(ctx: Option<&semantic::Context>) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if let Some(ctx) = ctx {
        items.extend(prop_variable_items(ctx));
        items.extend(builtin_expr_start_items());
        items.extend(ctx.functions.iter().map(|func| {
            let detail = Some(func.detail.clone());
            CompletionItem {
                label: func.name.clone(),
                kind: CompletionKind::Function,
                category: Some(func.category),
                insert_text: format!("{}()", func.name),
                primary_edit: None,
                cursor: None,
                additional_edits: Vec::new(),
                detail,
                is_disabled: false,
                disabled_reason: None,
                data: Some(CompletionData::Function {
                    name: func.name.clone(),
                }),
            }
        }));
    } else {
        items.extend(builtin_expr_start_items());
    }
    items
}

pub(super) fn after_atom_items(ctx: Option<&semantic::Context>) -> Vec<CompletionItem> {
    const OPS: [&str; 10] = ["==", "!=", ">=", ">", "<=", "<", "+", "-", "*", "/"];

    let mut items = Vec::new();
    items.extend(OPS.into_iter().map(|op| CompletionItem {
        label: op.to_string(),
        kind: CompletionKind::Operator,
        category: None,
        insert_text: op.to_string(),
        primary_edit: None,
        cursor: None,
        additional_edits: Vec::new(),
        detail: None,
        is_disabled: false,
        disabled_reason: None,
        data: None,
    }));

    items.extend(postfix_method_items(ctx, true));

    items
}

pub(super) fn after_dot_items(ctx: Option<&semantic::Context>) -> Vec<CompletionItem> {
    // In a member-access context, the `.` already exists in the source.
    postfix_method_items(ctx, false)
}

fn needs_trailing_space(name: &str) -> bool {
    matches!(name, "not" | "true" | "false")
}

fn builtin_expr_start_items() -> Vec<CompletionItem> {
    [("not", "not"), ("true", "true"), ("false", "false")]
        .into_iter()
        .map(|(label, insert_text)| {
            let insert_text = if needs_trailing_space(label) {
                format!("{insert_text} ")
            } else {
                insert_text.to_string()
            };
            CompletionItem {
                label: label.to_string(),
                kind: CompletionKind::Builtin,
                category: None,
                insert_text,
                primary_edit: None,
                cursor: None,
                additional_edits: Vec::new(),
                detail: None,
                is_disabled: false,
                disabled_reason: None,
                data: None,
            }
        })
        .collect()
}

fn postfix_method_items(ctx: Option<&semantic::Context>, insert_dot: bool) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let Some(ctx) = ctx else {
        return items;
    };
    let postfix_capable = semantic::postfix_capable_builtin_names();
    for func in &ctx.functions {
        if !postfix_capable.contains(func.name.as_str()) {
            continue;
        }
        let label = format!(".{}", func.name);
        let insert_text = if insert_dot {
            format!(".{}()", func.name)
        } else {
            format!("{}()", func.name)
        };
        items.push(CompletionItem {
            label,
            kind: CompletionKind::Operator,
            category: None,
            insert_text,
            primary_edit: None,
            cursor: None,
            additional_edits: Vec::new(),
            detail: None,
            is_disabled: false,
            disabled_reason: None,
            data: Some(CompletionData::PostfixMethod {
                name: func.name.clone(),
            }),
        });
    }

    items
}

fn prop_variable_items(ctx: &semantic::Context) -> Vec<CompletionItem> {
    if ctx.properties.is_empty() {
        return Vec::new();
    }
    let mut enabled = Vec::new();
    let mut disabled = Vec::new();
    for prop in &ctx.properties {
        let label = prop.name.clone();
        let insert_text = format!(r#"prop("{}")"#, prop.name);
        let item = CompletionItem {
            label,
            kind: CompletionKind::Property,
            category: None,
            insert_text,
            primary_edit: None,
            cursor: None,
            additional_edits: Vec::new(),
            detail: None,
            is_disabled: prop.disabled_reason.is_some(),
            disabled_reason: prop.disabled_reason.clone(),
            data: Some(CompletionData::PropExpr {
                property_name: prop.name.clone(),
            }),
        };
        if prop.disabled_reason.is_some() {
            disabled.push(item);
        } else {
            enabled.push(item);
        }
    }
    enabled.extend(disabled);
    enabled
}
