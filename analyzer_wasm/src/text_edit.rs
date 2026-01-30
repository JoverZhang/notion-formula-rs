//! Apply byte-based text edits and rebase a byte cursor.
//!
//! Edits are applied from back to front, and invalid ranges are skipped.

use analyzer::TextEdit;

/// Applies byte-offset text edits and rebases a byte cursor through them.
///
/// Edits are applied in descending order to avoid shifting later offsets.
/// An edit is skipped if it is out of bounds or not on UTF-8 char boundaries.
///
/// Cursor rules: edits before the cursor shift it by the byte delta; a cursor inside a replaced
/// range snaps to `start`.
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
