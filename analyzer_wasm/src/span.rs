//! Span conversion utilities for the WASM/JS boundary.
//!
//! The core analyzer uses spans expressed as **UTF-8 byte offsets** into the source text.
//! The WASM DTO surface uses **UTF-16 code unit offsets** (editor positions).
//!
//! All spans are **half-open** `[start, end)` (inclusive start, exclusive end).
//!
//! **Entry points**
//! - [`byte_span_to_utf16_span`]: convert an analyzer [`analyzer::Span`] to [`dto::v1::Span`].

use analyzer::Span;

use crate::dto::v1::Span as SpanView;
use crate::offsets::byte_offset_to_utf16_offset;

/// Convert a byte-based analyzer span into a UTF-16 DTO span.
///
/// Each endpoint is converted independently via [`byte_offset_to_utf16_offset`], so if either
/// endpoint is not a UTF-8 char boundary it is deterministically floored. This preserves
/// half-open semantics, but may shrink the span compared to the original byte range.
pub fn byte_span_to_utf16_span(source: &str, span: Span) -> SpanView {
    let start = byte_offset_to_utf16_offset(source, span.start as usize);
    let end = byte_offset_to_utf16_offset(source, span.end as usize);
    SpanView { start, end }
}
