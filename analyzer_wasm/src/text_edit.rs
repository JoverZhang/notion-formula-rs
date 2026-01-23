use analyzer::TextEdit;

/// Applies byte-offset text edits.
///
/// - Offsets are half-open `[start, end)` (end is exclusive).
/// - Edits are applied in descending order to avoid offset shifting.
pub fn apply_text_edits_bytes(source: &str, edits: &[TextEdit]) -> String {
    let mut sorted = edits.to_vec();
    sorted.sort_by(|a, b| {
        b.range
            .start
            .cmp(&a.range.start)
            .then(b.range.end.cmp(&a.range.end))
    });

    let mut updated = source.to_string();
    for edit in sorted {
        let start = edit.range.start as usize;
        let end = edit.range.end as usize;
        if start > end || end > updated.len() {
            continue;
        }
        if !updated.is_char_boundary(start) || !updated.is_char_boundary(end) {
            continue;
        }

        let mut next = String::with_capacity(updated.len() - (end - start) + edit.new_text.len());
        next.push_str(&updated[..start]);
        next.push_str(&edit.new_text);
        next.push_str(&updated[end..]);
        updated = next;
    }

    updated
}
