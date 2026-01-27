//! Conversion utilities for the WASM/JS boundary.
//!
//! This module is intentionally **stateless** and centralizes:
//! - Input conversion (e.g. parsing context JSON).
//! - UTF-16 ↔ byte offset bridging for editor-facing positions.
//! - DTO conversion (internal analyzer types → `dto::v1::*` views).
//!
//! It should not implement language/business logic; it only transforms data to/from
//! the types expected by the frontend and WASM exports.

use analyzer::SourceMap;
use analyzer::completion::{CompletionConfig, DEFAULT_PREFERRED_LIMIT};
use analyzer::semantic::{Context, FunctionCategory, Property, builtins_functions};
use analyzer::{Diagnostic, DiagnosticKind, ParseOutput, Span, Token, TokenKind};
use js_sys::Error as JsError;
use serde::Deserialize;
use wasm_bindgen::prelude::JsValue;

use crate::dto::v1::{
    AnalyzeResult, CompletionItemKind, CompletionItemView, CompletionOutputView,
    DiagnosticKindView, DiagnosticView, FunctionCategoryView, LineColView, SignatureHelpView,
    Span as SpanDto, SpanView, TextEditView, TokenView,
};
use crate::offsets::byte_offset_to_utf16_offset;
use crate::offsets::utf16_offset_to_byte;
use crate::span::byte_span_to_utf16_span;
use crate::text_edit::apply_text_edits_bytes_with_cursor;

pub struct Converter;

pub struct ParsedContext {
    pub ctx: Context,
    pub completion: CompletionConfig,
}

impl Converter {
    /// Parse the JS-provided context JSON into an analyzer `Context`.
    pub fn parse_context(context_json: &str) -> Result<ParsedContext, JsValue> {
        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct CompletionInput {
            #[serde(default = "default_preferred_limit")]
            preferred_limit: usize,
        }

        impl Default for CompletionInput {
            fn default() -> Self {
                Self {
                    preferred_limit: default_preferred_limit(),
                }
            }
        }

        fn default_preferred_limit() -> usize {
            DEFAULT_PREFERRED_LIMIT
        }

        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ContextInput {
            #[serde(default)]
            properties: Vec<Property>,
            #[serde(default)]
            completion: CompletionInput,
        }

        fn invalid_context_json_error() -> JsValue {
            JsValue::from(JsError::new("Invalid context JSON"))
        }

        let trimmed = context_json.trim();
        if trimmed.is_empty() {
            return Err(invalid_context_json_error());
        }

        let input: ContextInput =
            serde_json::from_str(trimmed).map_err(|_| invalid_context_json_error())?;
        Ok(ParsedContext {
            ctx: Context {
                properties: input.properties,
                functions: builtins_functions(),
            },
            completion: CompletionConfig {
                preferred_limit: input.completion.preferred_limit,
            },
        })
    }

    /// Convert a UTF-16 cursor offset (CodeMirror) to a byte offset (Rust strings).
    pub fn cursor_utf16_to_byte(source: &str, cursor_utf16: usize) -> usize {
        utf16_offset_to_byte(source, cursor_utf16)
    }

    /// Convert a UTF-16 offset to a `LineColView` (1-based line/col in the JS DTO).
    pub fn pos_to_line_col_view(source: &str, pos_utf16: u32) -> LineColView {
        let byte = utf16_offset_to_byte(source, pos_utf16 as usize);
        let (line, col) = SourceMap::new(source).line_col(byte as u32);
        LineColView {
            line: line as u32,
            col: col as u32,
        }
    }

    pub fn analyze_output(source: &str, output: ParseOutput) -> AnalyzeResult {
        let diagnostics = output
            .diagnostics
            .iter()
            .map(|d| Self::diagnostic_view(source, d))
            .collect();

        let tokens = output
            .tokens
            .iter()
            .filter(|t| !t.is_trivia())
            .map(|t| Self::token_view(source, t))
            .collect();

        let formatted = analyzer::format_expr(&output.expr, source, &output.tokens);

        AnalyzeResult {
            diagnostics,
            tokens,
            formatted,
        }
    }

    pub fn analyze_error(source: &str, diag: &Diagnostic) -> AnalyzeResult {
        AnalyzeResult {
            diagnostics: vec![Self::diagnostic_view(source, diag)],
            tokens: Vec::new(),
            formatted: String::new(),
        }
    }

    pub fn completion_output_view(
        source: &str,
        output: &analyzer::CompletionOutput,
    ) -> CompletionOutputView {
        let replace = Self::span_dto(source, output.replace);
        let signature_help = output.signature_help.as_ref().map(|sig| SignatureHelpView {
            label: sig.label.clone(),
            params: sig.params.clone(),
            active_param: sig.active_param,
        });

        let items = output
            .items
            .iter()
            .map(|item| Self::completion_item_view(source, output, item))
            .collect();

        CompletionOutputView {
            items,
            replace,
            signature_help,
            preferred_indices: output.preferred_indices.clone(),
        }
    }

    pub fn completion_item_view(
        source: &str,
        output: &analyzer::CompletionOutput,
        item: &analyzer::CompletionItem,
    ) -> CompletionItemView {
        let primary_edit_view = item.primary_edit.as_ref().map(|edit| TextEditView {
            range: Self::span_dto(source, edit.range),
            new_text: edit.new_text.clone(),
        });

        let additional_edits = item
            .additional_edits
            .iter()
            .map(|edit| TextEditView {
                range: Self::span_dto(source, edit.range),
                new_text: edit.new_text.clone(),
            })
            .collect::<Vec<_>>();

        let cursor_utf16 = item.primary_edit.as_ref().map(|primary_edit| {
            let mut edits = Vec::with_capacity(1 + item.additional_edits.len());
            edits.push(primary_edit.clone());
            edits.extend(item.additional_edits.iter().cloned());

            // The analyzer's `cursor` is intended to be a position in the *updated* document after
            // applying the primary edit (e.g., `if()` => inside `(`), but the additional edits may
            // shift that position. We account for shifts from additional edits that occur strictly
            // before the primary edit, without rebasing the cursor through the primary edit itself.
            let mut cursor_byte: i64 = i64::from(item.cursor.unwrap_or_else(|| {
                output
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

                    // Match the edit application validity checks: if an edit would be skipped due to
                    // invalid bounds or non-UTF8-boundary ranges, it must not shift the cursor.
                    if start_u32 > end_u32 || end > source.len() {
                        continue;
                    }
                    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
                        continue;
                    }

                    let replaced_len = end_u32.saturating_sub(start_u32) as i64;
                    let inserted_len = edit.new_text.len() as i64;
                    cursor_byte =
                        cursor_byte.saturating_add(inserted_len.saturating_sub(replaced_len));
                }
            }

            let (updated, _) = apply_text_edits_bytes_with_cursor(source, &edits, 0);
            let cursor_byte = usize::min(usize::try_from(cursor_byte).unwrap_or(0), updated.len());
            byte_offset_to_utf16_offset(&updated, cursor_byte)
        });

        CompletionItemView {
            label: item.label.clone(),
            kind: completion_kind_view(item.kind),
            category: item.category.map(function_category_view),
            insert_text: item.insert_text.clone(),
            primary_edit: primary_edit_view,
            cursor: cursor_utf16,
            additional_edits,
            detail: item.detail.clone(),
            is_disabled: item.is_disabled,
            disabled_reason: item.disabled_reason.clone(),
        }
    }

    pub fn diagnostic_view(source: &str, diag: &Diagnostic) -> DiagnosticView {
        DiagnosticView {
            kind: diagnostic_kind_view(&diag.kind),
            message: diag.message.clone(),
            span: Self::span_view(source, diag.span),
        }
    }

    pub fn token_view(source: &str, token: &Token) -> TokenView {
        let start = token.span.start as usize;
        let end = token.span.end as usize;
        let text = source.get(start..end).unwrap_or("").to_string();

        TokenView {
            kind: token_kind_string(&token.kind).to_string(),
            text,
            span: Self::span_view(source, token.span),
        }
    }

    pub fn span_view(source: &str, span: Span) -> SpanView {
        let range = byte_span_to_utf16_span(source, span);
        SpanView { range }
    }

    pub fn span_dto(source: &str, span: Span) -> SpanDto {
        byte_span_to_utf16_span(source, span)
    }
}

fn diagnostic_kind_view(kind: &DiagnosticKind) -> DiagnosticKindView {
    match kind {
        DiagnosticKind::Error => DiagnosticKindView::Error,
    }
}

fn completion_kind_view(kind: analyzer::CompletionKind) -> CompletionItemKind {
    use analyzer::CompletionKind::*;
    match kind {
        Function => CompletionItemKind::Function,
        Builtin => CompletionItemKind::Builtin,
        Property => CompletionItemKind::Property,
        Operator => CompletionItemKind::Operator,
    }
}

fn function_category_view(category: FunctionCategory) -> FunctionCategoryView {
    match category {
        FunctionCategory::General => FunctionCategoryView::General,
        FunctionCategory::Text => FunctionCategoryView::Text,
        FunctionCategory::Number => FunctionCategoryView::Number,
        FunctionCategory::Date => FunctionCategoryView::Date,
        FunctionCategory::People => FunctionCategoryView::People,
        FunctionCategory::List => FunctionCategoryView::List,
        FunctionCategory::Special => FunctionCategoryView::Special,
    }
}

fn token_kind_string(kind: &TokenKind) -> &'static str {
    use TokenKind::*;
    use analyzer::LitKind;

    match kind {
        Lt => "Lt",
        Le => "Le",
        EqEq => "EqEq",
        Ne => "Ne",
        Ge => "Ge",
        Gt => "Gt",
        AndAnd => "AndAnd",
        OrOr => "OrOr",
        Bang => "Bang",
        Plus => "Plus",
        Minus => "Minus",
        Star => "Star",
        Slash => "Slash",
        Percent => "Percent",
        Caret => "Caret",
        Dot => "Dot",
        Comma => "Comma",
        Colon => "Colon",
        Pound => "Pound",
        Question => "Question",
        OpenParen => "OpenParen",
        CloseParen => "CloseParen",
        Literal(lit) => match lit.kind {
            LitKind::Bool => "Bool",
            LitKind::Number => "Number",
            LitKind::String => "String",
        },
        Ident(_) => "Ident",
        DocComment(..) => "DocComment",
        LineComment(_) => "LineComment",
        BlockComment(_) => "BlockComment",
        Newline => "Newline",
        Eof => "Eof",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_view_extracts_text_and_converts_span_to_utf16() {
        let source = "😀+1";
        let output = analyzer::analyze(source).expect("expected parse ok");
        let token = output
            .tokens
            .iter()
            .find(|t| !t.is_trivia() && !matches!(t.kind, TokenKind::Eof))
            .expect("expected a non-trivia token");

        let view = Converter::token_view(source, token);
        assert_eq!(view.text, "😀");
        assert_eq!(view.span.range.start, 0);
        assert_eq!(view.span.range.end, 2);
    }

    #[test]
    fn diagnostic_view_converts_span_to_utf16() {
        let source = "1 +";
        let output = analyzer::analyze(source).expect("expected parse ok with diagnostics");
        let diag = output
            .diagnostics
            .first()
            .expect("expected at least one diagnostic");

        let view = Converter::diagnostic_view(source, diag);
        assert_eq!(view.kind, DiagnosticKindView::Error);
        assert_eq!(view.span.range.start, 2);
        assert_eq!(view.span.range.end, 3);
    }
}
