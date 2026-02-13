//! Small IDE helpers for editor integrations.
//! Uses analyzer spans as UTF-8 byte offsets, with half-open ranges `[start, end)`.
//! Some helpers also work in token indices; those APIs say so explicitly.
//! Use `completion::complete` for completion + signature help.

pub mod completion;
pub mod display;
mod edit;
pub mod format;

pub use edit::{ApplyResult, IdeError, apply_edits, ide_format};

/// Completion payload used by `help`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionResult {
    pub items: Vec<completion::CompletionItem>,
    pub replace: crate::Span,
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
    ctx: &crate::semantic::Context,
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
