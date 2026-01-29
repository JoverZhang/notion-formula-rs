//! Text edit application + cursor rebasing (byte offsets).
//!
//! The analyzer produces edits in terms of **UTF-8 byte offsets** into the original source.
//! This module applies those edits and rebases a byte cursor through the applied changes.
//!
//! All ranges are **half-open** `[start, end)` (inclusive start, exclusive end).
//!
//! **Entry points**
//! - [`apply_text_edits_bytes_with_cursor`]: apply edits (descending) and rebase a cursor.

use analyzer::TextEdit;

/// Applies byte-offset text edits and rebases a byte cursor through them.
///
/// - Offsets are half-open `[start, end)` (end is exclusive).
/// - Edits are applied in descending order to avoid offset shifting.
/// - Cursor rebasing is computed only for edits that are actually applied (valid bounds and UTF-8
///   char boundaries).
///
/// **Validity / clamping**
/// - Edits with `start > end`, `end > source.len()`, or non-UTF-8 char-boundary endpoints are
///   skipped (and therefore do not affect the cursor).
/// - The returned cursor is always a valid byte offset within the updated string.
///
/// **Cursor semantics (deterministic)**
/// - If an applied edit lies strictly before the cursor (`edit.range.end <= cursor`), the cursor
///   is shifted by the edit's byte-length delta.
/// - If the cursor lies strictly inside the replaced range (`start < cursor < end`), it snaps to
///   `start`.
/// - If the cursor is exactly at `start` or `end`, it is left unchanged (half-open boundary
///   semantics).
pub fn apply_text_edits_bytes_with_cursor(
    source: &str,
    edits: &[TextEdit],
    cursor: u32,
) -> (String, u32) {
    let mut sorted = edits.to_vec();
    sorted.sort_by(|a, b| {
        b.range
            .start
            .cmp(&a.range.start)
            .then(b.range.end.cmp(&a.range.end))
    });

    let mut updated = source.to_string();
    let mut cursor = cursor;

    for edit in sorted {
        let start_u32 = edit.range.start;
        let end_u32 = edit.range.end;
        let start = start_u32 as usize;
        let end = end_u32 as usize;

        if start_u32 > end_u32 || end > updated.len() {
            continue;
        }
        if !updated.is_char_boundary(start) || !updated.is_char_boundary(end) {
            continue;
        }

        let replaced_len = end_u32.saturating_sub(start_u32);
        let inserted_len = edit.new_text.len() as u32;
        let delta = inserted_len as i64 - replaced_len as i64;

        // Rebase cursor in original coordinates through this edit.
        if end_u32 <= cursor {
            cursor = if delta >= 0 {
                cursor.saturating_add(delta as u32)
            } else {
                cursor.saturating_sub((-delta) as u32)
            };
        } else if start_u32 < cursor && cursor < end_u32 {
            // Cursor was inside replaced range => snap to start boundary deterministically.
            cursor = start_u32;
        }

        let mut next = String::with_capacity(updated.len() - (end - start) + edit.new_text.len());
        next.push_str(&updated[..start]);
        next.push_str(&edit.new_text);
        next.push_str(&updated[end..]);
        updated = next;
    }

    (updated, cursor)
}
