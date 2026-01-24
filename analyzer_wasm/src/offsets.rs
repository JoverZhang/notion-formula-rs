/// JS/editor boundary uses UTF-16 code units (CodeMirror positions).
/// Ranges are half-open `[start, end)`; `end` is exclusive.
pub fn utf16_offset_to_byte(source: &str, utf16: usize) -> usize {
    if utf16 == 0 {
        return 0;
    }

    let mut u16_count = 0usize;
    for (byte_idx, ch) in source.char_indices() {
        if u16_count >= utf16 {
            return byte_idx;
        }
        u16_count += ch.len_utf16();
    }

    source.len()
}

/// Converts a byte offset into a UTF-16 code-unit offset (CodeMirror positions).
///
/// If `byte` is not a char boundary, it is treated as the previous boundary.
pub fn byte_offset_to_utf16_offset(source: &str, byte: usize) -> u32 {
    if byte == 0 {
        return 0;
    }

    let mut u16_count = 0u32;
    for (byte_idx, ch) in source.char_indices() {
        if byte_idx >= byte {
            return u16_count;
        }
        u16_count = u16_count.saturating_add(ch.len_utf16() as u32);
    }

    source.encode_utf16().count() as u32
}
