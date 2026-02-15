//! IDE-facing API surface extracted from `analyzer`.
//!
//! This crate currently forwards to `analyzer` so callers can switch dependencies
//! before implementation is fully moved here.

pub use analyzer::TextEdit;
pub use analyzer::{HelpResult, IdeError};

pub type ApplyResult = analyzer::IdeApplyResult;
pub type CompletionResult = analyzer::IdeCompletionResult;

pub mod completion {
    pub use analyzer::{
        CompletionConfig, CompletionData, CompletionItem, CompletionKind, SignatureHelp,
    };
}

pub mod display {
    pub use analyzer::ide::display::DisplaySegment;
}

pub use completion::{CompletionConfig, CompletionData, CompletionItem, CompletionKind, SignatureHelp};

/// Format a source string and rebase a byte cursor.
pub fn format(source: &str, cursor_byte: u32) -> Result<ApplyResult, IdeError> {
    analyzer::ide_format(source, cursor_byte)
}

/// Apply byte edits and rebase a byte cursor.
pub fn apply_edits(
    source: &str,
    edits: Vec<TextEdit>,
    cursor_byte: u32,
) -> Result<ApplyResult, IdeError> {
    analyzer::ide_apply_edits(source, edits, cursor_byte)
}

/// Compute completion and signature-help at a byte cursor.
pub fn help(
    source: &str,
    cursor_byte: usize,
    ctx: &analyzer::semantic::Context,
    config: CompletionConfig,
) -> HelpResult {
    analyzer::ide_help(source, cursor_byte, ctx, config)
}
