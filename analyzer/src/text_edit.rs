use crate::lexer::Span;

/// A single text edit in byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Span,
    pub new_text: String,
}
