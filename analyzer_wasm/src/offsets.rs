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
