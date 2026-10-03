//! JS-facing DTO types for `analyzer_wasm`.
//!
//! Spans and offsets use UTF-16 code units and are half-open `[start, end)`.
pub mod engine;
pub mod v1;
