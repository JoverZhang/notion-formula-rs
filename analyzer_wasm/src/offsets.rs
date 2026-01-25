/// JS/editor boundary uses UTF-16 code units (CodeMirror positions).
///
/// All offsets/ranges are half-open `[start, end)`; `end` is exclusive.
pub fn utf16_offset_to_byte(source: &str, utf16: usize) -> usize {
    let utf16_len = source.encode_utf16().count();
    let utf16 = utf16.min(utf16_len);

    if utf16 == 0 {
        return 0;
    }

    let mut u16_count = 0usize;
    for (byte_idx, ch) in source.char_indices() {
        if u16_count >= utf16 {
            return byte_idx;
        }

        let next = u16_count.saturating_add(ch.len_utf16());
        // If `utf16` falls inside a surrogate pair (or otherwise inside this scalar's UTF-16
        // encoding), floor to the start of this Unicode scalar (a Rust `char` boundary).
        if next > utf16 {
            return byte_idx;
        }
        u16_count = next;
    }

    source.len()
}

/// Converts a byte offset into a UTF-16 code-unit offset (CodeMirror positions).
///
/// If `byte` is not a char boundary (i.e. inside a UTF-8 codepoint), it is floored to the
/// previous Unicode scalar boundary.
pub fn byte_offset_to_utf16_offset(source: &str, byte: usize) -> u32 {
    if byte == 0 {
        return 0;
    }

    let mut u16_count = 0u32;
    for (byte_idx, ch) in source.char_indices() {
        if byte <= byte_idx {
            return u16_count;
        }

        let ch_end = byte_idx.saturating_add(ch.len_utf8());
        if byte < ch_end {
            // `byte` falls inside this scalar's UTF-8 encoding => floor to this scalar start.
            return u16_count;
        }

        u16_count = u16_count.saturating_add(ch.len_utf16() as u32);
    }

    source.encode_utf16().count() as u32
}

#[cfg(test)]
mod tests {
    use super::{byte_offset_to_utf16_offset, utf16_offset_to_byte};

    #[test]
    fn utf16_offset_to_byte_floors_inside_surrogate_pair() {
        let source = "😀a";
        // Inside the emoji's surrogate pair => floor to emoji start (byte 0).
        assert_eq!(utf16_offset_to_byte(source, 1), 0);
        // End of emoji (2 UTF-16 code units) => byte after emoji.
        assert_eq!(utf16_offset_to_byte(source, 2), "😀".len());
        // End of string => source.len().
        assert_eq!(
            utf16_offset_to_byte(source, source.encode_utf16().count()),
            source.len()
        );
    }

    #[test]
    fn byte_offset_to_utf16_offset_floors_inside_utf8_codepoint() {
        let source = "😀a";
        // Byte 1 is inside the emoji's UTF-8 encoding => floor to UTF-16 offset 0.
        assert_eq!(byte_offset_to_utf16_offset(source, 1), 0);
        // Byte after emoji => UTF-16 offset 2.
        assert_eq!(byte_offset_to_utf16_offset(source, "😀".len()), 2);
    }
}
