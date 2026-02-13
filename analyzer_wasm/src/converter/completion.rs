use crate::converter::Converter;
use crate::converter::shared::span_dto;
use crate::dto::v1::{
    CompletionItem, CompletionItemKind, CompletionResult as CompletionResultDto, DisplaySegment,
    HelpResult as HelpResultDto, SignatureHelp, SignatureItem, TextEdit,
};

impl Converter {
    pub fn help_output_view(source: &str, output: &analyzer::ide::HelpResult) -> HelpResultDto {
        let replace = span_dto(source, output.completion.replace);
        let signature_help = output.signature_help.as_ref().map(|sig| SignatureHelp {
            signatures: sig
                .signatures
                .iter()
                .map(|s| SignatureItem {
                    segments: s.segments.iter().map(display_segment_view).collect(),
                })
                .collect(),
            active_signature: sig.active_signature,
            active_parameter: sig.active_parameter,
        });

        let items = output
            .completion
            .items
            .iter()
            .map(|item| completion_item_view(source, &output.completion, item))
            .collect();

        HelpResultDto {
            completion: CompletionResultDto {
                items,
                replace,
                preferred_indices: output.completion.preferred_indices.clone(),
            },
            signature_help,
        }
    }
}

fn completion_item_view(
    source: &str,
    completion: &analyzer::ide::CompletionResult,
    item: &analyzer::CompletionItem,
) -> CompletionItem {
    let primary_edit_view = item.primary_edit.as_ref().map(|edit| TextEdit {
        range: span_dto(source, edit.range),
        new_text: edit.new_text.clone(),
    });

    let additional_edits = item
        .additional_edits
        .iter()
        .map(|edit| TextEdit {
            range: span_dto(source, edit.range),
            new_text: edit.new_text.clone(),
        })
        .collect::<Vec<_>>();

    let cursor_utf16 = item.primary_edit.as_ref().map(|primary_edit| {
        let mut edits = Vec::with_capacity(1 + item.additional_edits.len());
        edits.push(primary_edit.clone());
        edits.extend(item.additional_edits.iter().cloned());
        edits.sort_by(|a, b| {
            a.range
                .start
                .cmp(&b.range.start)
                .then(a.range.end.cmp(&b.range.end))
        });

        // The analyzer's `cursor` is intended to be a position in the updated document after
        // applying the primary edit, but additional edits may shift that position.
        let mut cursor_byte: i64 = i64::from(item.cursor.unwrap_or_else(|| {
            completion
                .replace
                .start
                .saturating_add(primary_edit.new_text.len() as u32)
        }));

        let primary_start = primary_edit.range.start;
        for edit in &item.additional_edits {
            if edit.range.end <= primary_start {
                let start_u32 = edit.range.start;
                let end_u32 = edit.range.end;
                let start = start_u32 as usize;
                let end = end_u32 as usize;

                // Match edit validity checks: skipped edits must not shift the cursor.
                if start_u32 > end_u32 || end > source.len() {
                    continue;
                }
                if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
                    continue;
                }

                let replaced_len = end_u32.saturating_sub(start_u32) as i64;
                let inserted_len = edit.new_text.len() as i64;
                cursor_byte = cursor_byte.saturating_add(inserted_len.saturating_sub(replaced_len));
            }
        }

        let (updated, _) = analyzer::apply_text_edits_bytes_with_cursor(source, &edits, 0);
        let cursor_byte = usize::min(usize::try_from(cursor_byte).unwrap_or(0), updated.len());
        Converter::byte_offset_to_utf16_offset(&updated, cursor_byte)
    });

    CompletionItem {
        label: item.label.clone(),
        kind: completion_kind_view(item.kind),
        insert_text: item.insert_text.clone(),
        primary_edit: primary_edit_view,
        cursor: cursor_utf16,
        additional_edits,
        detail: item.detail.clone(),
        is_disabled: item.is_disabled,
        disabled_reason: item.disabled_reason.clone(),
    }
}

fn display_segment_view(seg: &analyzer::ide::display::DisplaySegment) -> DisplaySegment {
    use analyzer::ide::display::DisplaySegment as S;
    match seg {
        S::Name { text } => DisplaySegment::Name { text: text.clone() },
        S::Punct { text } => DisplaySegment::Punct { text: text.clone() },
        S::Separator { text } => DisplaySegment::Separator { text: text.clone() },
        S::Ellipsis => DisplaySegment::Ellipsis,
        S::Arrow { text } => DisplaySegment::Arrow { text: text.clone() },
        S::Param {
            name,
            ty,
            param_index,
        } => DisplaySegment::Param {
            name: name.clone(),
            ty: ty.clone(),
            param_index: *param_index,
        },
        S::ReturnType { text } => DisplaySegment::ReturnType { text: text.clone() },
    }
}

fn completion_kind_view(kind: analyzer::CompletionKind) -> CompletionItemKind {
    use analyzer::CompletionKind::*;
    match kind {
        FunctionGeneral => CompletionItemKind::FunctionGeneral,
        FunctionText => CompletionItemKind::FunctionText,
        FunctionNumber => CompletionItemKind::FunctionNumber,
        FunctionDate => CompletionItemKind::FunctionDate,
        FunctionPeople => CompletionItemKind::FunctionPeople,
        FunctionList => CompletionItemKind::FunctionList,
        FunctionSpecial => CompletionItemKind::FunctionSpecial,
        Builtin => CompletionItemKind::Builtin,
        Property => CompletionItemKind::Property,
        Operator => CompletionItemKind::Operator,
    }
}
