use analyzer::Span;

use crate::dto::v1::Utf16Span;
use crate::offsets::byte_offset_to_utf16_offset;

pub fn byte_span_to_utf16_span(source: &str, span: Span) -> Utf16Span {
    let start = byte_offset_to_utf16_offset(source, span.start as usize);
    let end = byte_offset_to_utf16_offset(source, span.end as usize);
    Utf16Span { start, end }
}
