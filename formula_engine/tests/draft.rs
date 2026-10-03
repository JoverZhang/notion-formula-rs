use std::collections::HashMap;
use std::fmt::Write;

use analyzer::{CommentKind, TokenKind};
use formula_engine::{
    Column, ColumnData, CompletionConfig, CompletionItem, CompletionKind, CreateDraftError,
    DiagnosticId, DraftVersion, EvaluateInput, ExpressionUpdate, FormulaDefinition, FormulaDraft,
    FormulaDraftState, FormulaEdit, FormulaEngine, FormulaEngineState, FormulaSchema,
    FormulaStatus, NullBuffer, PropertyDefinition, PropertyId, PropertyState, RuntimeContext, Span,
    TextEdit, TextOffset, UpdateExpressionError, UpdateExpressionResult, ValueType,
};

fn definition(id: &str, expression: &str) -> FormulaDefinition {
    FormulaDefinition {
        id: id.into(),
        expression: expression.into(),
    }
}

fn formula(id: &str, expression: &str) -> PropertyDefinition {
    PropertyDefinition::Formula(definition(id, expression))
}

fn input(id: &str, ty: ValueType) -> PropertyDefinition {
    PropertyDefinition::Input { id: id.into(), ty }
}

fn engine(properties: Vec<PropertyDefinition>) -> FormulaEngine {
    FormulaEngine::new(FormulaSchema { properties }).expect("valid schema")
}

fn draft<'a>(engine: &'a FormulaEngine, id: &str, expression: &str) -> FormulaDraft<'a> {
    engine
        .create_draft(definition(id, expression))
        .unwrap_or_else(|_| panic!("nonempty ID must allow creating a draft"))
}

fn replace(draft: &mut FormulaDraft<'_>, expression: &str) -> UpdateExpressionResult {
    draft
        .update_expression(ExpressionUpdate::Replace(expression.into()))
        .unwrap_or_else(|_| panic!("Replace must allow invalid expressions"))
}

fn text_edit(start: u32, end: u32, new_text: &str) -> TextEdit {
    TextEdit {
        range: Span { start, end },
        new_text: new_text.into(),
    }
}

fn apply(
    draft: &mut FormulaDraft<'_>,
    edits: Vec<TextEdit>,
    cursor: usize,
) -> UpdateExpressionResult {
    draft
        .update_expression(ExpressionUpdate::Edits {
            edit: FormulaEdit {
                base_version: draft.state().version,
                edits,
            },
            cursor: TextOffset(cursor),
        })
        .unwrap_or_else(|_| panic!("valid current edits must succeed"))
}

#[derive(Debug, PartialEq, Eq)]
struct StateSnapshot {
    version: u64,
    definition: FormulaDefinition,
    output_type: ValueType,
    diagnostics: Vec<(String, Span, String)>,
    tokens: Vec<(TokenKind, Span)>,
}

fn snapshot(state: &FormulaDraftState) -> StateSnapshot {
    StateSnapshot {
        version: state.version.0,
        definition: state.definition.clone(),
        output_type: state.output_type.clone(),
        diagnostics: state
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.id.0.clone(),
                    diagnostic.span,
                    diagnostic.message.clone(),
                )
            })
            .collect(),
        tokens: state
            .tokens
            .iter()
            .map(|token| (token.kind.clone(), token.span))
            .collect(),
    }
}

fn property_item<'a>(items: &'a [CompletionItem], id: &str) -> &'a CompletionItem {
    items
        .iter()
        .find(|item| item.kind == CompletionKind::Property && item.label == id)
        .unwrap_or_else(|| panic!("missing property completion for {id}"))
}

fn assert_disabled(item: &CompletionItem) {
    assert!(item.is_disabled, "{} must be disabled", item.label);
    assert!(item.disabled_reason.as_ref().is_some_and(|s| !s.is_empty()));
    assert!(item.primary_edit.is_none());
    assert!(item.cursor.is_none());
}

fn evaluation_input() -> EvaluateInput {
    EvaluateInput {
        row_ids: vec!["row-1".into(), "row-2".into()],
        columns: HashMap::from([
            (
                "text".into(),
                Column::String(ColumnData {
                    values: vec!["ha".into(), "go".into()],
                    validity: NullBuffer::new_valid(2),
                }),
            ),
            (
                "count".into(),
                Column::Number(ColumnData {
                    values: vec![2.0, 3.0],
                    validity: NullBuffer::new_valid(2),
                }),
            ),
        ]),
        runtime: RuntimeContext {
            now: 1_700_000_123_456,
            time_zone: "+08:00".into(),
        },
        formula_ids: vec!["saved".into()],
    }
}

#[test]
fn edit_extract_save_and_evaluate_with_multiple_live_drafts() {
    let mut engine = engine(vec![
        input("text", ValueType::String),
        input("count", ValueType::Number),
        formula("saved", r#"repeat(prop("text"), prop("count"))"#),
    ]);
    let properties_before = engine.properties();
    let mut first = draft(&engine, "saved", r#"repeat(prop("text"), prop("count"))"#);
    let second = draft(&engine, "copy", r#"prop("saved")"#);
    let edited = replace(&mut first, r#"repeat(prop("text"), prop("count") + 1)"#);
    assert_eq!(edited.state.output_type, ValueType::String);
    assert!(edited.state.diagnostics.is_empty());
    assert_eq!(snapshot(first.state()), snapshot(&edited.state));
    assert_eq!(second.state().output_type, ValueType::String);
    assert!(second.state().diagnostics.is_empty());
    assert_eq!(engine.properties(), properties_before);

    // An immutable Engine operation remains available while both Drafts borrow it.
    let before = engine.evaluate(&evaluation_input()).unwrap();
    let before_output = before.formulas[&PropertyId::from("saved")]
        .as_ref()
        .unwrap();
    let Column::String(before_column) = &before_output.column else {
        panic!("expected a string output");
    };
    assert_eq!(before_column.values, ["haha", "gogogo"]);

    let first_definition = first.into_definition();
    let second_definition = second.into_definition();
    assert_eq!(first_definition.id, "saved".into());
    assert_eq!(second_definition.id, "copy".into());
    engine
        .upsert(PropertyDefinition::Formula(first_definition))
        .unwrap();
    engine
        .upsert(PropertyDefinition::Formula(second_definition))
        .unwrap();
    let mut request = evaluation_input();
    request.formula_ids.push("copy".into());
    let after = engine.evaluate(&request).unwrap();
    for id in ["saved", "copy"] {
        let output = after.formulas[&PropertyId::from(id)].as_ref().unwrap();
        let Column::String(column) = &output.column else {
            panic!("expected a string output for {id}");
        };
        assert_eq!(column.values, ["hahaha", "gogogogo"]);
        assert_eq!(output.output_type, ValueType::String);
        assert!(output.errors.is_empty());
    }
    assert_eq!(before_column.values, ["haha", "gogogo"]);
    assert!(matches!(engine.state(), FormulaEngineState::AllReady));
}

#[test]
fn creating_a_draft_validates_only_the_id_and_never_saves_it() {
    let engine = engine(vec![input("existing", ValueType::Number)]);
    let before = engine.properties();
    assert!(matches!(
        engine.create_draft(definition("", "1")),
        Err(CreateDraftError::EmptyId)
    ));
    for (id, expression) in [
        ("new", "1 +"),
        ("existing", r#"prop("existing")"#),
        ("missing", r#"prop("absent")"#),
        ("semantic", r#"abs("text")"#),
    ] {
        let draft = draft(&engine, id, expression);
        assert_eq!(draft.state().definition, definition(id, expression));
        assert!(!draft.state().diagnostics.is_empty());
        assert_eq!(engine.properties(), before);
    }
    assert!(engine.property(&"new".into()).is_none());
}

#[test]
fn partial_syntax_types_comments_newlines_and_eof_are_observable() {
    let engine = engine(vec![]);
    let source = "// 中文🙂\n[1 2] /* 保留 */\n";
    let mut draft = draft(&engine, "candidate", source);
    assert_eq!(
        draft.state().output_type,
        ValueType::List(Box::new(ValueType::Number))
    );
    assert!(!draft.state().diagnostics.is_empty());
    assert!(draft.state().tokens.iter().any(|token| {
        matches!(token.kind, TokenKind::DocComment(CommentKind::Line, _))
            && &source[token.span.start as usize..token.span.end as usize] == "// 中文🙂"
    }));
    assert!(draft.state().tokens.iter().any(|token| {
        matches!(token.kind, TokenKind::DocComment(CommentKind::Block, _))
            && &source[token.span.start as usize..token.span.end as usize] == "/* 保留 */"
    }));
    assert_eq!(
        draft
            .state()
            .tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Newline))
            .count(),
        2
    );
    let eof = draft.state().tokens.last().unwrap();
    assert!(matches!(eof.kind, TokenKind::Eof));
    assert_eq!(
        eof.span,
        Span {
            start: source.len() as u32,
            end: source.len() as u32
        }
    );
    for diagnostic in &draft.state().diagnostics {
        assert!(diagnostic.span.start <= diagnostic.span.end);
        assert!(source.is_char_boundary(diagnostic.span.start as usize));
        assert!(source.is_char_boundary(diagnostic.span.end as usize));
        assert!(!diagnostic.message.is_empty());
    }

    let empty_list = replace(&mut draft, "[]");
    assert_eq!(
        empty_list.state.output_type,
        ValueType::List(Box::new(ValueType::Unknown))
    );
    assert!(empty_list.state.diagnostics.is_empty());
    let empty = replace(&mut draft, "");
    assert_eq!(empty.state.output_type, ValueType::Unknown);
    assert_eq!(empty.cursor.0, 0);
    assert_eq!(empty.state.tokens.len(), 1);
    assert!(matches!(empty.state.tokens[0].kind, TokenKind::Eof));
    assert_eq!(empty.state.tokens[0].span, Span { start: 0, end: 0 });
}

#[test]
fn versions_and_owned_update_snapshots_follow_text_and_edit_rules() {
    let engine = engine(vec![]);
    let mut draft = draft(&engine, "candidate", "1");
    let initial = draft.state().version.0;
    let same = replace(&mut draft, "1");
    assert_eq!(same.state.version.0, initial);
    assert_eq!(same.cursor.0, 1);
    let changed = replace(&mut draft, r#""中文🙂""#);
    assert_eq!(changed.state.version.0, initial + 1);
    assert_eq!(changed.cursor.0, r#""中文🙂""#.len());
    assert_eq!(changed.state.output_type, ValueType::String);

    let empty_edits = apply(&mut draft, vec![], 1);
    assert_eq!(empty_edits.state.version.0, initial + 2);
    assert_eq!(empty_edits.cursor.0, 1);
    let same_text_edit = apply(&mut draft, vec![text_edit(1, 4, "中")], 1);
    assert_eq!(same_text_edit.state.version.0, initial + 3);
    assert_eq!(same_text_edit.state.definition.expression, r#""中文🙂""#);
    let list = replace(&mut draft, "[[1], [2]]");
    assert_eq!(list.state.version.0, initial + 4);
    assert_eq!(
        list.state.output_type,
        ValueType::List(Box::new(ValueType::List(Box::new(ValueType::Number))))
    );

    // Results own their state and do not track subsequent edits.
    assert_eq!(changed.state.version.0, initial + 1);
    assert_eq!(changed.state.definition.expression, r#""中文🙂""#);
    assert_eq!(changed.state.output_type, ValueType::String);
    assert_eq!(same.state.output_type, ValueType::Number);
    assert_eq!(snapshot(draft.state()), snapshot(&list.state));
}

#[test]
fn completion_and_signature_help_use_the_current_draft_version() {
    let engine = engine(vec![input("amount", ValueType::Number)]);
    let mut draft = draft(&engine, "candidate", "amo");
    assert_eq!(CompletionConfig::default().preferred_limit, 5);
    let help = draft.help(TextOffset(3), CompletionConfig::default());
    let completion = property_item(&help.completion.items, "amount");
    assert!(!completion.is_disabled);
    assert!(
        help.completion
            .preferred_indices
            .iter()
            .any(|index| help.completion.items[*index].label == "amount")
    );
    let edit = FormulaEdit {
        base_version: help.base_version,
        edits: std::iter::once(completion.primary_edit.clone().unwrap())
            .chain(completion.additional_edits.clone())
            .collect(),
    };
    let result = draft
        .update_expression(ExpressionUpdate::Edits {
            edit,
            cursor: TextOffset(3),
        })
        .unwrap_or_else(|_| panic!("current completion edits must apply"));
    assert_eq!(result.state.definition.expression, r#"prop("amount")"#);
    assert_eq!(result.state.output_type, ValueType::Number);
    assert!(result.state.diagnostics.is_empty());

    replace(&mut draft, "repeat(\"中🙂\", ");
    let help = draft.help(
        TextOffset(draft.state().definition.expression.len()),
        CompletionConfig::default(),
    );
    assert_eq!(help.base_version.0, draft.state().version.0);
    let signature = help.signature_help.unwrap();
    assert_eq!(signature.active_signature, 0);
    assert_eq!(signature.active_parameter, 1);
    assert_eq!(signature.signatures.len(), 1);
    assert!(!signature.signatures[0].segments.is_empty());
    replace(&mut draft, "amo");
    assert!(
        draft
            .help(TextOffset(3), CompletionConfig { preferred_limit: 0 })
            .completion
            .preferred_indices
            .is_empty()
    );
}

#[test]
fn help_tolerates_invalid_utf8_and_out_of_bounds_cursor_offsets() {
    let engine = engine(vec![input("amount", ValueType::Number)]);
    let source = r#""中🙂""#;
    let draft = draft(&engine, "candidate", source);
    let before = snapshot(draft.state());
    for cursor in [2, 5, source.len() + 1, usize::MAX] {
        let help = draft.help(TextOffset(cursor), CompletionConfig::default());
        assert_eq!(help.base_version.0, before.version);
        assert!(
            help.completion
                .preferred_indices
                .iter()
                .all(|index| *index < help.completion.items.len())
        );
        assert_eq!(snapshot(draft.state()), before);
    }
}

#[test]
fn stale_completion_edits_are_rejected_without_changing_state() {
    let engine = engine(vec![input("amount", ValueType::Number)]);
    let mut draft = draft(&engine, "candidate", "amo");
    let help = draft.help(TextOffset(3), CompletionConfig::default());
    let completion = property_item(&help.completion.items, "amount");
    let edit = FormulaEdit {
        base_version: help.base_version,
        edits: vec![completion.primary_edit.clone().unwrap()],
    };
    replace(&mut draft, "am");
    let before = snapshot(draft.state());
    assert!(matches!(
        draft.update_expression(ExpressionUpdate::Edits {
            edit,
            cursor: TextOffset(2)
        }),
        Err(UpdateExpressionError::VersionMismatch)
    ));
    assert_eq!(snapshot(draft.state()), before);
}

#[test]
fn quick_fixes_are_suggestions_and_diagnostic_ids_expire_even_if_text_returns() {
    let engine = engine(vec![]);
    let mut draft = draft(&engine, "candidate", "[1,2,]");
    let before = snapshot(draft.state());
    let diagnostic = draft
        .state()
        .diagnostics
        .iter()
        .find(|diagnostic| !draft.quick_fixes(&diagnostic.id).is_empty())
        .expect("trailing comma must have a parser recovery suggestion");
    let obsolete_id = DiagnosticId(diagnostic.id.0.clone());
    let mut fixes = draft.quick_fixes(&obsolete_id);
    assert!(!fixes.is_empty());
    assert!(
        fixes
            .iter()
            .all(|fix| !fix.title.is_empty() && fix.edit.base_version.0 == before.version)
    );
    assert_eq!(snapshot(draft.state()), before);
    let help = draft.help(TextOffset(0), CompletionConfig::default());
    assert_eq!(help.base_version.0, before.version);
    assert_eq!(snapshot(draft.state()), before);
    assert!(
        draft
            .quick_fixes(&DiagnosticId("__unknown_diagnostic__".into()))
            .is_empty()
    );

    let stale_edit = fixes.pop().unwrap().edit;
    replace(&mut draft, "[1,2]");
    let repaired = snapshot(draft.state());
    assert!(draft.quick_fixes(&obsolete_id).is_empty());
    assert!(matches!(
        draft.update_expression(ExpressionUpdate::Edits {
            edit: stale_edit,
            cursor: TextOffset(5),
        }),
        Err(UpdateExpressionError::VersionMismatch)
    ));
    assert_eq!(snapshot(draft.state()), repaired);

    replace(&mut draft, "[1,2,]");
    assert!(draft.quick_fixes(&obsolete_id).is_empty());
    let current_id = DiagnosticId(
        draft
            .state()
            .diagnostics
            .iter()
            .find(|diagnostic| !draft.quick_fixes(&diagnostic.id).is_empty())
            .unwrap()
            .id
            .0
            .clone(),
    );
    assert_ne!(current_id.0, obsolete_id.0);
    let current_fix = draft.quick_fixes(&current_id).pop().unwrap();
    let version = draft.state().version.0;
    let result = draft
        .update_expression(ExpressionUpdate::Edits {
            edit: current_fix.edit,
            cursor: TextOffset(6),
        })
        .unwrap_or_else(|_| panic!("current parser recovery suggestion must apply"));
    assert_eq!(result.state.version.0, version + 1);
    assert_eq!(result.state.definition.expression, "[1,2]");
    assert!(result.state.diagnostics.is_empty());
    assert_eq!(
        result.state.output_type,
        ValueType::List(Box::new(ValueType::Number))
    );
    assert!(draft.quick_fixes(&current_id).is_empty());
}

#[test]
fn successful_noop_edits_expire_diagnostic_ids() {
    let engine = engine(vec![]);
    let mut draft = draft(&engine, "candidate", "[1,2,]");
    let ids: Vec<_> = draft
        .state()
        .diagnostics
        .iter()
        .map(|diagnostic| DiagnosticId(diagnostic.id.0.clone()))
        .collect();
    assert!(ids.iter().any(|id| !draft.quick_fixes(id).is_empty()));
    apply(&mut draft, vec![], 0);
    assert_eq!(draft.state().definition.expression, "[1,2,]");
    for id in ids {
        assert!(draft.quick_fixes(&id).is_empty());
    }
    assert!(
        draft
            .state()
            .diagnostics
            .iter()
            .any(|diagnostic| !draft.quick_fixes(&diagnostic.id).is_empty())
    );
}

#[test]
fn diagnostic_ids_from_another_draft_are_not_current() {
    let engine = engine(vec![]);
    let first = draft(&engine, "first", "[1,2,]");
    let second = draft(&engine, "second", "[1,2,]");
    assert_eq!(first.state().version.0, second.state().version.0);
    for diagnostic in &first.state().diagnostics {
        assert!(second.quick_fixes(&diagnostic.id).is_empty());
    }
    assert!(
        second
            .state()
            .diagnostics
            .iter()
            .any(|diagnostic| !second.quick_fixes(&diagnostic.id).is_empty())
    );
}

#[test]
fn draft_public_results_snapshot() {
    let engine = engine(vec![
        input("candidate", ValueType::Number),
        formula("dependent", r#"prop("candidate")"#),
        formula("safe", "7"),
    ]);
    let source = "// 中🙂\n[1,2,]\n";
    let partial = draft(&engine, "partial", source);
    let state = partial.state();
    let mut out = String::new();
    writeln!(
        out,
        "partial: version={} output={:?}",
        state.version.0, state.output_type
    )
    .unwrap();
    // Preserve public diagnostic and token order without recording opaque IDs.
    for (index, diagnostic) in state.diagnostics.iter().enumerate() {
        writeln!(
            out,
            "diagnostic[{index}]: {}..{} {}",
            diagnostic.span.start, diagnostic.span.end, diagnostic.message
        )
        .unwrap();
        for fix in partial.quick_fixes(&diagnostic.id) {
            writeln!(
                out,
                "quickfix: {} version={}",
                fix.title, fix.edit.base_version.0
            )
            .unwrap();
            for edit in fix.edit.edits {
                writeln!(
                    out,
                    "edit: {}..{} => {:?}",
                    edit.range.start, edit.range.end, edit.new_text
                )
                .unwrap();
            }
        }
    }
    for token in &state.tokens {
        let kind = match &token.kind {
            TokenKind::DocComment(kind, _) => format!("Comment({kind:?})"),
            TokenKind::Literal(literal) => format!("Literal({:?})", literal.kind),
            TokenKind::Newline => "Newline".into(),
            TokenKind::Eof => "Eof".into(),
            kind => kind.to_str().unwrap().into(),
        };
        writeln!(
            out,
            "token: {kind} {}..{} {:?}",
            token.span.start,
            token.span.end,
            &source[token.span.start as usize..token.span.end as usize]
        )
        .unwrap();
    }
    let candidate = draft(&engine, "candidate", "can");
    let help = candidate.help(TextOffset(3), CompletionConfig::default());
    writeln!(out, "completion: version={}", help.base_version.0).unwrap();
    for id in ["candidate", "dependent", "safe"] {
        let item = property_item(&help.completion.items, id);
        let preferred = help
            .completion
            .preferred_indices
            .iter()
            .any(|index| help.completion.items[*index].label == id);
        writeln!(
            out,
            "{id}: disabled={} reason={} primary={} cursor={} preferred={preferred}",
            item.is_disabled,
            item.disabled_reason.is_some(),
            item.primary_edit.is_some(),
            item.cursor.is_some()
        )
        .unwrap();
    }
    assert_eq!(out, include_str!("draft.snap"));
}

#[test]
fn formatting_allows_semantic_errors_but_rejects_parser_and_lexer_errors() {
    let engine = engine(vec![]);
    let source = "/* 中文🙂 */ abs(\"x\")+prop(\"missing\")";
    let mut draft = draft(&engine, "candidate", source);
    assert!(!draft.state().diagnostics.is_empty());
    let before = snapshot(draft.state());
    let edit = draft
        .format_edits()
        .unwrap_or_else(|_| panic!("semantic errors must allow formatting"));
    assert_eq!(snapshot(draft.state()), before);
    assert_eq!(edit.base_version.0, before.version);
    assert_eq!(edit.edits.len(), 1);
    assert_eq!(
        edit.edits[0].range,
        Span {
            start: 0,
            end: source.len() as u32
        }
    );
    assert!(edit.edits[0].new_text.ends_with('\n'));
    assert!(edit.edits[0].new_text.contains("中文🙂"));
    let result = draft
        .update_expression(ExpressionUpdate::Edits {
            edit,
            cursor: TextOffset(source.len()),
        })
        .unwrap_or_else(|_| panic!("format edit must apply"));
    assert_eq!(result.cursor.0, result.state.definition.expression.len());
    assert!(!result.state.diagnostics.is_empty());
    let again = draft
        .format_edits()
        .unwrap_or_else(|_| panic!("formatted semantic errors remain formattable"));
    assert_eq!(again.edits[0].new_text, result.state.definition.expression);
    for source in ["1 +", "\"unterminated"] {
        replace(&mut draft, source);
        let before = snapshot(draft.state());
        assert!(draft.format_edits().is_err());
        assert_eq!(snapshot(draft.state()), before);
    }
}

#[test]
fn utf8_invalid_cursors_ranges_and_overlaps_fail_atomically() {
    let engine = engine(vec![]);
    let source = r#""中🙂文""#;
    let mut draft = draft(&engine, "candidate", source);
    let before = snapshot(draft.state());
    for cursor in [2, 5, source.len() + 1, usize::MAX] {
        assert!(matches!(
            draft.update_expression(ExpressionUpdate::Edits {
                edit: FormulaEdit {
                    base_version: draft.state().version,
                    edits: vec![text_edit(1, 4, "X")]
                },
                cursor: TextOffset(cursor),
            }),
            Err(UpdateExpressionError::InvalidCursor)
        ));
        assert_eq!(snapshot(draft.state()), before, "cursor={cursor}");
    }
    for edit in [
        text_edit(2, 4, "X"),
        text_edit(4, 6, "X"),
        text_edit(8, 4, "X"),
        text_edit(0, source.len() as u32 + 1, "X"),
        text_edit(u32::MAX, u32::MAX, "X"),
    ] {
        assert!(matches!(
            draft.update_expression(ExpressionUpdate::Edits {
                edit: FormulaEdit {
                    base_version: draft.state().version,
                    edits: vec![edit]
                },
                cursor: TextOffset(0),
            }),
            Err(UpdateExpressionError::InvalidEditRange)
        ));
        assert_eq!(snapshot(draft.state()), before);
    }
    assert!(matches!(
        draft.update_expression(ExpressionUpdate::Edits {
            edit: FormulaEdit {
                base_version: draft.state().version,
                edits: vec![text_edit(4, 11, "Y"), text_edit(1, 8, "X")],
            },
            cursor: TextOffset(0),
        }),
        Err(UpdateExpressionError::OverlappingEdits)
    ));
    assert_eq!(snapshot(draft.state()), before);
    assert!(matches!(
        draft.update_expression(ExpressionUpdate::Edits {
            edit: FormulaEdit {
                base_version: DraftVersion(before.version + 1),
                edits: vec![]
            },
            cursor: TextOffset(0),
        }),
        Err(UpdateExpressionError::VersionMismatch)
    ));
    assert_eq!(snapshot(draft.state()), before);
}

#[test]
fn unsorted_utf8_edits_same_offset_inserts_and_cursor_rebasing() {
    let engine = engine(vec![]);
    let source = r#""中🙂文""#;
    let mut draft = draft(&engine, "candidate", source);
    let edits = vec![text_edit(8, 11, "B"), text_edit(1, 4, "字字")];
    let result = apply(&mut draft, edits.clone(), source.len());
    assert_eq!(result.state.definition.expression, r#""字字🙂B""#);
    assert_eq!(result.cursor.0, r#""字字🙂B""#.len());
    assert_eq!(result.state.output_type, ValueType::String);
    assert!(result.state.diagnostics.is_empty());

    replace(&mut draft, source);
    let inside = apply(&mut draft, vec![text_edit(1, 8, "X")], 4);
    assert_eq!(inside.cursor.0, 1);
    replace(&mut draft, source);
    let at_start = apply(&mut draft, vec![text_edit(1, 8, "X")], 1);
    assert_eq!(at_start.cursor.0, 1);
    replace(&mut draft, source);
    let at_end = apply(&mut draft, vec![text_edit(1, 8, "X")], 8);
    assert_eq!(at_end.cursor.0, 2);

    replace(&mut draft, r#""AB""#);
    let inserts = apply(
        &mut draft,
        vec![text_edit(2, 2, "中"), text_edit(2, 2, "🙂")],
        2,
    );
    assert_eq!(inserts.state.definition.expression, r#""A中🙂B""#);
    assert_eq!(inserts.cursor.0, 2 + "中🙂".len());
    replace(&mut draft, source);
    let adjacent = apply(
        &mut draft,
        vec![text_edit(4, 8, "Y"), text_edit(1, 4, "X")],
        4,
    );
    assert_eq!(adjacent.state.definition.expression, r#""XY文""#);
    assert_eq!(adjacent.cursor.0, 2);
}

#[test]
fn direct_and_indirect_cycles_use_the_candidate_even_when_replacing_an_input() {
    let engine = engine(vec![
        input("candidate", ValueType::Number),
        formula("dependent", r#"prop("candidate")"#),
        formula("transitive", r#"prop("dependent")"#),
        formula("safe", "7"),
    ]);
    let before = engine.properties();
    let mut draft = draft(&engine, "candidate", r#"prop("candidate")"#);
    assert!(!draft.state().diagnostics.is_empty());
    assert!(draft.state().diagnostics.iter().any(
        |diagnostic| diagnostic.span.end as usize <= draft.state().definition.expression.len()
    ));
    replace(&mut draft, r#"prop("transitive")"#);
    assert!(!draft.state().diagnostics.is_empty());
    replace(&mut draft, r#""edited""#);
    assert_eq!(draft.state().output_type, ValueType::String);
    assert!(draft.state().diagnostics.is_empty());
    assert_eq!(engine.properties(), before);

    replace(&mut draft, "can");
    let help = draft.help(TextOffset(3), CompletionConfig::default());
    for id in ["candidate", "dependent", "transitive"] {
        assert_disabled(property_item(&help.completion.items, id));
        assert!(
            !help
                .completion
                .preferred_indices
                .iter()
                .any(|index| help.completion.items[*index].label == id)
        );
    }
    assert!(!property_item(&help.completion.items, "safe").is_disabled);
}

#[test]
fn overlay_repair_and_dependency_type_changes_do_not_modify_saved_states() {
    let mut engine = engine(vec![
        formula("candidate", "1 +"),
        formula("copy", r#"prop("candidate")"#),
        formula("length", r#"length(prop("copy"))"#),
    ]);
    let saved_before = engine.properties();
    let mut candidate = draft(&engine, "candidate", r#""new type""#);
    assert_eq!(candidate.state().output_type, ValueType::String);
    assert!(candidate.state().diagnostics.is_empty());
    let help = candidate.help(TextOffset(0), CompletionConfig::default());
    assert_disabled(property_item(&help.completion.items, "copy"));
    assert_disabled(property_item(&help.completion.items, "length"));
    replace(&mut candidate, r#"prop("copy")"#);
    assert!(!candidate.state().diagnostics.is_empty());
    replace(&mut candidate, r#""new type""#);
    assert_eq!(engine.properties(), saved_before);
    let extracted = candidate.into_definition();
    let mut affected = engine
        .upsert(PropertyDefinition::Formula(extracted))
        .unwrap()
        .affected_formulas;
    affected.sort();
    assert_eq!(
        affected,
        ["candidate".into(), "copy".into(), "length".into()]
    );
    for (id, expected) in [
        ("candidate", ValueType::String),
        ("copy", ValueType::String),
        ("length", ValueType::Number),
    ] {
        assert!(
            matches!(engine.property(&id.into()), Some(PropertyState::Formula(state))
            if state.status == (FormulaStatus::Ready { output_type: expected }))
        );
    }
    assert!(matches!(engine.state(), FormulaEngineState::AllReady));
    let fresh = draft(&engine, "consumer", r#"length(prop("copy"))"#);
    assert_eq!(fresh.state().output_type, ValueType::Number);
    assert!(fresh.state().diagnostics.is_empty());
    assert!(saved_before.iter().all(|property| matches!(property, PropertyState::Formula(state) if state.status == FormulaStatus::NotReady)));
}

#[test]
fn unrelated_not_ready_formulas_do_not_poison_valid_drafts_or_help() {
    let engine = engine(vec![
        formula("syntax_error", "1 +"),
        formula("semantic_error", r#"abs("bad")"#),
        formula("missing_dependency", r#"prop("absent")"#),
        formula("cycle", r#"prop("cycle")"#),
        formula("safe", "9"),
    ]);
    let mut draft = draft(&engine, "candidate", r#"prop("safe") + 1"#);
    assert_eq!(draft.state().output_type, ValueType::Number);
    assert!(draft.state().diagnostics.is_empty());
    let help = draft.help(TextOffset(0), CompletionConfig::default());
    assert!(!property_item(&help.completion.items, "safe").is_disabled);
    for id in [
        "syntax_error",
        "semantic_error",
        "missing_dependency",
        "cycle",
    ] {
        assert_disabled(property_item(&help.completion.items, id));
        let expression = format!("prop(\"{id}\")");
        let result = replace(&mut draft, &expression);
        assert!(
            !result.state.diagnostics.is_empty(),
            "actual dependency {id} must be diagnosed"
        );
        assert!(
            result
                .state
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.span.end as usize <= expression.len())
        );
    }
    replace(&mut draft, r#"prop("safe") + 1"#);
    assert!(draft.state().diagnostics.is_empty());
}
