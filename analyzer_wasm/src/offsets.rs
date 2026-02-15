//! Convert between UTF-16 offsets (editors) and UTF-8 byte offsets (Rust).
//!
//! Inputs are clamped and floored to valid boundaries, so these helpers never panic.

use analyzer::{Span as ByteSpan, TextEdit as ByteTextEdit};
use ide::IdeError;

use crate::converter::Converter;
use crate::dto::v1::{Span as Utf16Span, TextEdit as Utf16TextEdit};

impl Converter {
    /// Convert a UTF-16 code unit offset into a UTF-8 byte offset.
    ///
    /// Out-of-range values are clamped. If the offset lands inside a scalar's UTF-16 encoding (for
    /// example, inside a surrogate pair), it is floored to the scalar start.
    pub fn utf16_to_8_offset(source: &str, utf16: usize) -> usize {
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
            // If `utf16` falls inside this scalar's UTF-16 encoding, floor to the scalar start.
            if next > utf16 {
                return byte_idx;
            }
            u16_count = next;
        }

        source.len()
    }

    /// Convert a UTF-8 byte offset into a UTF-16 code unit offset.
    ///
    /// If `byte` is not a UTF-8 char boundary, it is floored to the previous boundary.
    /// If `byte` is past the end, this returns the UTF-16 length of `source`.
    pub fn utf8_to_16_offset(source: &str, byte: usize) -> u32 {
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
}

pub fn utf16_to_8_cursor(source: &str, cursor_utf16: u32) -> Result<usize, IdeError> {
    let utf16_len = source.encode_utf16().count();
    let cursor_utf16 = cursor_utf16 as usize;
    if cursor_utf16 > utf16_len {
        return Err(IdeError::InvalidCursor);
    }

    let cursor_utf8 = Converter::utf16_to_8_offset(source, cursor_utf16);
    if !source.is_char_boundary(cursor_utf8) {
        return Err(IdeError::InvalidCursor);
    }

    Ok(cursor_utf8)
}

pub fn utf16_to_8_text_edits(
    source: &str,
    text_edits: Vec<Utf16TextEdit>,
) -> Result<Vec<ByteTextEdit>, IdeError> {
    let utf16_len = source.encode_utf16().count();

    let mut utf8_text_edits = Vec::with_capacity(text_edits.len());
    for edit in text_edits {
        let Utf16Span { start, end } = edit.range;
        let start_utf16 = start as usize;
        let end_utf16 = end as usize;

        if end_utf16 < start_utf16 || end_utf16 > utf16_len {
            return Err(IdeError::InvalidEditRange);
        }

        let start_utf8 = Converter::utf16_to_8_offset(source, start_utf16);
        let end_utf8 = Converter::utf16_to_8_offset(source, end_utf16);

        if end_utf8 < start_utf8 {
            return Err(IdeError::InvalidEditRange);
        }
        if !source.is_char_boundary(start_utf8) || !source.is_char_boundary(end_utf8) {
            return Err(IdeError::InvalidEditRange);
        }

        utf8_text_edits.push(ByteTextEdit {
            range: ByteSpan {
                start: start_utf8 as u32,
                end: end_utf8 as u32,
            },
            new_text: edit.new_text,
        });
    }

    Ok(utf8_text_edits)
}

#[cfg(test)]
mod tests {
    use ide::IdeError;

    use crate::converter::Converter;
    use crate::dto::v1::{Span as Utf16Span, TextEdit as Utf16TextEdit};
    use crate::offsets::{utf16_to_8_cursor, utf16_to_8_text_edits};

    #[test]
    fn utf16_to_8_offset_floors_inside_scalar_encoding() {
        let source = "😀a";
        // Inside the emoji's surrogate pair => floor to emoji start (byte 0).
        assert_eq!(Converter::utf16_to_8_offset(source, 1), 0);
        // End of emoji (2 UTF-16 code units) => byte after emoji.
        assert_eq!(Converter::utf16_to_8_offset(source, 2), "😀".len());
        // End of string => source.len().
        assert_eq!(
            Converter::utf16_to_8_offset(source, source.encode_utf16().count()),
            source.len()
        );
    }

    #[test]
    fn utf8_to_16_offset_floors_inside_utf8_encoding() {
        let source = "😀a";
        // Byte 1 is inside the emoji's UTF-8 encoding => floor to UTF-16 offset 0.
        assert_eq!(Converter::utf8_to_16_offset(source, 1), 0);
        // Byte after emoji => UTF-16 offset 2.
        assert_eq!(Converter::utf8_to_16_offset(source, "😀".len()), 2);
    }

    #[test]
    fn utf16_to_8_cursor_rejects_out_of_bounds() {
        let source = "abc";
        let err = utf16_to_8_cursor(source, 4).expect_err("expected out-of-bounds cursor");
        assert_eq!(err, IdeError::InvalidCursor);
    }

    #[test]
    fn utf16_to_8_text_edits_rejects_out_of_bounds_range() {
        let source = "abc";
        let text_edits = vec![Utf16TextEdit {
            range: Utf16Span { start: 1, end: 4 },
            new_text: "x".to_string(),
        }];

        let err =
            utf16_to_8_text_edits(source, text_edits).expect_err("expected invalid edit range");
        assert_eq!(err, IdeError::InvalidEditRange);
    }
}
