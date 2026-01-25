use analyzer::Span;

use crate::dto::v1::Span as SpanView;
use crate::offsets::byte_offset_to_utf16_offset;

pub fn byte_span_to_utf16_span(source: &str, span: Span) -> SpanView {
    let start = byte_offset_to_utf16_offset(source, span.start as usize);
    let end = byte_offset_to_utf16_offset(source, span.end as usize);
    SpanView { start, end }
}
