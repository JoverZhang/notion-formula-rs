//! Convert analyzer spans (byte offsets) to DTO spans (UTF-16 code units).
//!
//! Each end is converted separately and floored to a valid boundary.

use analyzer::Span;

use crate::dto::v1::Span as SpanView;
use crate::offsets::byte_offset_to_utf16_offset;

/// Convert a byte-span to a UTF-16 span.
///
/// If an endpoint is not on a UTF-8 char boundary, it is floored to the previous boundary.
pub fn byte_span_to_utf16_span(source: &str, span: Span) -> SpanView {
    let start = byte_offset_to_utf16_offset(source, span.start as usize);
    let end = byte_offset_to_utf16_offset(source, span.end as usize);
    SpanView { start, end }
}
