//! IDE helpers for editor integrations.
//!
//! Coordinates are UTF-8 byte offsets (`[start, end)`), matching `analyzer`.

mod completion;
mod context;
mod display;
mod edit;
mod format;
mod signature;
mod text_edit;

use analyzer::semantic;
use analyzer::{Span, Token, TokenKind};
use context::{CursorContext, PositionKind};

pub use analyzer::TextEdit;
pub use completion::{CompletionConfig, CompletionData, CompletionItem, CompletionKind};
pub use display::DisplaySegment;
pub use edit::{ApplyResult, IdeError, apply_edits};
pub use signature::{SignatureHelp, SignatureItem};
pub use text_edit::apply_text_edits_bytes_with_cursor;

/// Completion payload used by `help`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionResult {
    pub items: Vec<completion::CompletionItem>,
    pub replace: analyzer::Span,
    pub preferred_indices: Vec<usize>,
}

/// Combined completion + signature help payload for IDE integrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpResult {
    pub completion: CompletionResult,
    pub signature_help: Option<SignatureHelp>,
}

/// Compute completion and signature-help at a byte cursor.
pub fn help(
    source: &str,
    cursor: usize,
    ctx: &semantic::Context,
    config: completion::CompletionConfig,
) -> HelpResult {
    HelpSession::new(source, cursor, ctx, config).run()
}

struct HelpSession<'a> {
    source: &'a str,
    cursor: u32,
    ctx: &'a semantic::Context,
    config: completion::CompletionConfig,
    tokens: Vec<Token>,
}

struct CompletionDraft {
    items: Vec<CompletionItem>,
    replace: Span,
}

impl<'a> HelpSession<'a> {
    fn new(
        source: &'a str,
        cursor: usize,
        ctx: &'a semantic::Context,
        config: completion::CompletionConfig,
    ) -> Self {
        Self {
            source,
            cursor: u32::try_from(cursor).unwrap_or(u32::MAX),
            ctx,
            config,
            tokens: analyzer::analyze_syntax(source).tokens,
        }
    }

    fn run(self) -> HelpResult {
        // 1) Detect call/position/query context at the cursor.
        let cursor_ctx = context::detect_cursor_context(
            self.source,
            self.tokens.as_slice(),
            self.cursor,
            self.ctx,
        );

        // 2) Compute signature help from call context.
        let signature_help = signature::compute_signature_help_if_in_call(
            self.source,
            self.tokens.as_slice(),
            self.cursor,
            self.ctx,
            cursor_ctx.call_ctx.as_ref(),
        );

        // 3) Build raw completion items for the position kind.
        let draft = self.build_completion_draft(&cursor_ctx);

        // 4) Attach primary edits and cursor positions.
        let mut items = draft.items;
        completion::attach_primary_edits(draft.replace, &mut items);

        // 5) Rank by query (sort + filter).
        if let Some(query) = cursor_ctx.query.as_deref() {
            completion::rank_by_query(query, &mut items, cursor_ctx.position_kind);
        }

        // 6) Pick preferred indices.
        let preferred_indices = match cursor_ctx.query.as_deref() {
            Some(query) => {
                completion::preferred_indices(&items, query, self.config.preferred_limit)
            }
            None => Vec::new(),
        };

        HelpResult {
            completion: CompletionResult {
                items,
                replace: draft.replace,
                preferred_indices,
            },
            signature_help,
        }
    }

    fn build_completion_draft(&self, cursor_ctx: &CursorContext) -> CompletionDraft {
        let default_replace = Span {
            start: self.cursor,
            end: self.cursor,
        };

        if self
            .tokens
            .iter()
            .all(|token| matches!(token.kind, TokenKind::Eof))
        {
            let items = if self.cursor == 0 {
                completion::expr_start_items(self.ctx)
            } else {
                Vec::new()
            };

            return CompletionDraft {
                items,
                replace: default_replace,
            };
        }

        let items = match cursor_ctx.position_kind {
            PositionKind::NeedExpr => {
                let expected =
                    context::expected_call_arg_ty(cursor_ctx.call_ctx.as_ref(), self.ctx);
                let mut items = completion::expr_start_items(self.ctx);
                if expected.is_some() {
                    completion::apply_type_ranking(&mut items, expected, self.ctx);
                }
                items
            }
            PositionKind::AfterAtom => completion::after_atom_items(self.ctx),
            PositionKind::AfterDot => {
                completion::after_dot_items(self.ctx, &self.infer_postfix_receiver_ty())
            }
            PositionKind::None => Vec::new(),
        };

        CompletionDraft {
            items,
            replace: cursor_ctx.replace,
        }
    }

    fn infer_postfix_receiver_ty(&self) -> semantic::Ty {
        let Some(dot_idx) =
            context::postfix_member_access_dot_index(self.tokens.as_slice(), self.cursor)
        else {
            return semantic::Ty::Unknown;
        };
        let Some(dot_token) = self.tokens.get(dot_idx) else {
            return semantic::Ty::Unknown;
        };
        let Ok(dot_start) = usize::try_from(dot_token.span.start) else {
            return semantic::Ty::Unknown;
        };
        if dot_start > self.source.len() || !self.source.is_char_boundary(dot_start) {
            return semantic::Ty::Unknown;
        }

        let receiver_source = self.source[..dot_start].trim_end();
        if receiver_source.is_empty() {
            return semantic::Ty::Unknown;
        }

        let parsed = analyzer::analyze_syntax(receiver_source);

        let mut map = analyzer::TypeMap::default();
        analyzer::infer_expr_with_map(&parsed.expr, self.ctx, &mut map)
    }
}

/// Format a source string and rebase a byte cursor.
pub fn format(source: &str, cursor_byte: u32) -> Result<ApplyResult, IdeError> {
    edit::ide_format(source, cursor_byte)
}

#[cfg(test)]
mod tests;
