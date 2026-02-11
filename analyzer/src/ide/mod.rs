//! Small IDE helpers for editor integrations.
//! Uses analyzer spans as UTF-8 byte offsets, with half-open ranges `[start, end)`.
//! Some helpers also work in token indices; those APIs say so explicitly.
//! Use `completion::complete` for completion + signature help.

pub mod completion;
pub mod display;
pub mod format;
pub mod quick_fix;
