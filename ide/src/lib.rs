//! IDE helpers for editor integrations.
//!
//! Coordinates are UTF-8 byte offsets (`[start, end)`), matching `analyzer`.

mod completion;
mod display;
mod edit;
mod format;
mod text_edit;

pub use analyzer::TextEdit;
pub use completion::{
    CompletionConfig, CompletionData, CompletionItem, CompletionKind, SignatureHelp, SignatureItem,
};
pub use display::DisplaySegment;
pub use edit::{ApplyResult, IdeError, apply_edits};
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
    pub signature_help: Option<completion::SignatureHelp>,
}

/// Compute completion and signature-help at a byte cursor.
pub fn help(
    source: &str,
    cursor: usize,
    ctx: &analyzer::semantic::Context,
    config: completion::CompletionConfig,
) -> HelpResult {
    let output = completion::complete(source, cursor, Some(ctx), config);

    HelpResult {
        completion: CompletionResult {
            items: output.items,
            replace: output.replace,
            preferred_indices: output.preferred_indices,
        },
        signature_help: output.signature_help,
    }
}

/// Format a source string and rebase a byte cursor.
pub fn format(source: &str, cursor_byte: u32) -> Result<ApplyResult, IdeError> {
    edit::ide_format(source, cursor_byte)
}

#[cfg(test)]
mod tests;
