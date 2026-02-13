use crate::apply_text_edits_bytes_with_cursor;
use crate::{Diagnostic, DiagnosticCode, Span as ByteSpan, TextEdit as ByteTextEdit};

/// Result payload for IDE edit operations in byte coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub source: String,
    pub cursor: u32,
}

/// Deterministic IDE operation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdeError {
    FormatError,
    InvalidCursor,
    InvalidEditRange,
    OverlappingEdits,
}

impl IdeError {
    pub fn message(self) -> &'static str {
        match self {
            IdeError::FormatError => "Format error",
            IdeError::InvalidCursor => "Invalid cursor",
            IdeError::InvalidEditRange => "Invalid edit range",
            IdeError::OverlappingEdits => "Overlapping edits",
        }
    }
}

/// Format a source string and rebase a byte cursor through the full-document replacement edit.
pub fn ide_format(source: &str, cursor: u32) -> Result<ApplyResult, IdeError> {
    let output = crate::analyze_syntax(source);

    if has_syntax_errors(&output.diagnostics) {
        return Err(IdeError::FormatError);
    }

    let source_len = u32::try_from(source.len()).map_err(|_| IdeError::InvalidEditRange)?;
    let formatted = crate::format_expr(&output.expr, source, &output.tokens);
    let full_document_edit = ByteTextEdit {
        range: ByteSpan {
            start: 0,
            end: source_len,
        },
        new_text: formatted,
    };

    apply_sorted_byte_edits(source, vec![full_document_edit], cursor)
}

/// Apply byte edits in source coordinates and rebase a byte cursor.
pub fn apply_edits(
    source: &str,
    mut edits: Vec<ByteTextEdit>,
    cursor: u32,
) -> Result<ApplyResult, IdeError> {
    edits.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then(a.range.end.cmp(&b.range.end))
    });

    apply_sorted_byte_edits(source, edits, cursor)
}

fn has_syntax_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|d| matches!(d.code, DiagnosticCode::LexError | DiagnosticCode::Parse(_)))
}

fn apply_sorted_byte_edits(
    source: &str,
    edits: Vec<ByteTextEdit>,
    cursor: u32,
) -> Result<ApplyResult, IdeError> {
    validate_cursor(source, cursor)?;
    validate_sorted_non_overlapping_edits(source, &edits)?;

    let (updated_source, cursor_after) = apply_text_edits_bytes_with_cursor(source, &edits, cursor);
    Ok(ApplyResult {
        source: updated_source,
        cursor: cursor_after,
    })
}

fn validate_cursor(source: &str, cursor: u32) -> Result<(), IdeError> {
    let cursor = cursor as usize;
    if cursor > source.len() || !source.is_char_boundary(cursor) {
        return Err(IdeError::InvalidCursor);
    }
    Ok(())
}

fn validate_sorted_non_overlapping_edits(
    source: &str,
    edits: &[ByteTextEdit],
) -> Result<(), IdeError> {
    let mut prev_end = 0u32;
    let source_len = u32::try_from(source.len()).map_err(|_| IdeError::InvalidEditRange)?;

    for (index, edit) in edits.iter().enumerate() {
        if edit.range.end < edit.range.start || edit.range.end > source_len {
            return Err(IdeError::InvalidEditRange);
        }

        if !source.is_char_boundary(edit.range.start as usize)
            || !source.is_char_boundary(edit.range.end as usize)
        {
            return Err(IdeError::InvalidEditRange);
        }

        if index > 0 && edit.range.start < prev_end {
            return Err(IdeError::OverlappingEdits);
        }

        prev_end = edit.range.end;
    }

    Ok(())
}
