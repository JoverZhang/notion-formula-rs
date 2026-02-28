//! Completion + signature help for editors.
//! All coordinates are UTF-8 byte offsets into the input `text`.
//! Spans are half-open ranges `[start, end)`.

pub use crate::TextEdit;
use crate::signature::SignatureHelp;
use analyzer::Span;
use analyzer::semantic;

mod items;
mod matchers;
mod ranking;

pub(crate) use items::{after_atom_items, after_dot_items, expr_start_items};
pub(crate) use ranking::{apply_type_ranking, attach_primary_edits, preferred_indices, rank_by_query};

/// Default for `CompletionConfig.preferred_limit`.
pub const DEFAULT_PREFERRED_LIMIT: usize = 5;

/// Configuration knobs for `complete`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionConfig {
    /// Max length of `CompletionOutput.preferred_indices` (0 disables it).
    pub preferred_limit: usize,
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            preferred_limit: DEFAULT_PREFERRED_LIMIT,
        }
    }
}

/// Result of a completion query at a byte cursor.
///
/// `replace` and all edit ranges are UTF-8 byte spans, half-open `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionOutput {
    pub items: Vec<CompletionItem>,
    pub replace: Span,
    pub signature_help: Option<SignatureHelp>,
    /// Indices into `items` for the UI default selection.
    pub preferred_indices: Vec<usize>,
}

/// One completion candidate for an editor UI.
///
/// If `cursor` is set, it is a desired byte offset in the updated document after applying
/// the primary edit.
///
/// Use [`CompletionItem::new`] to construct with sensible defaults, then chain
/// builder methods (`.with_detail()`, `.with_data()`, etc.) to customise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub insert_text: String,
    pub primary_edit: Option<TextEdit>,
    pub cursor: Option<u32>,
    pub additional_edits: Vec<TextEdit>,
    pub detail: Option<String>,
    pub is_disabled: bool,
    pub disabled_reason: Option<String>,
    pub data: Option<CompletionData>,
}

impl CompletionItem {
    /// Creates a new completion item with the given label and kind.
    ///
    /// `insert_text` defaults to a clone of `label`; all optional fields start
    /// as `None` / empty / `false`.
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        let label = label.into();
        Self {
            insert_text: label.clone(),
            label,
            kind,
            primary_edit: None,
            cursor: None,
            additional_edits: Vec::new(),
            detail: None,
            is_disabled: false,
            disabled_reason: None,
            data: None,
        }
    }

    pub fn with_insert_text(mut self, text: impl Into<String>) -> Self {
        self.insert_text = text.into();
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_data(mut self, data: CompletionData) -> Self {
        self.data = Some(data);
        self
    }

    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.is_disabled = true;
        self.disabled_reason = Some(reason.into());
        self
    }
}

/// High-level bucket for UI grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    FunctionGeneral,
    FunctionText,
    FunctionNumber,
    FunctionDate,
    FunctionPeople,
    FunctionList,
    FunctionSpecial,
    Builtin,
    Property,
    Operator,
}

impl CompletionKind {
    pub fn is_function(self) -> bool {
        matches!(
            self,
            Self::FunctionGeneral
                | Self::FunctionText
                | Self::FunctionNumber
                | Self::FunctionDate
                | Self::FunctionPeople
                | Self::FunctionList
                | Self::FunctionSpecial
        )
    }
}

impl From<semantic::FunctionCategory> for CompletionKind {
    fn from(value: semantic::FunctionCategory) -> Self {
        match value {
            semantic::FunctionCategory::General => Self::FunctionGeneral,
            semantic::FunctionCategory::Text => Self::FunctionText,
            semantic::FunctionCategory::Number => Self::FunctionNumber,
            semantic::FunctionCategory::Date => Self::FunctionDate,
            semantic::FunctionCategory::People => Self::FunctionPeople,
            semantic::FunctionCategory::List => Self::FunctionList,
            semantic::FunctionCategory::Special => Self::FunctionSpecial,
        }
    }
}

/// Extra metadata used for cursor placement and type ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionData {
    Function { name: String },
    PropExpr { property_name: String },
    PostfixMethod { name: String },
}

/// Computes completion items and signature help at a cursor position.
///
/// `cursor` is a UTF-8 byte offset into `text`.
#[allow(dead_code)]
pub fn complete(
    text: &str,
    cursor: usize,
    ctx: &semantic::Context,
    config: CompletionConfig,
) -> CompletionOutput {
    let help = crate::help(text, cursor, ctx, config);

    CompletionOutput {
        items: help.completion.items,
        replace: help.completion.replace,
        signature_help: help.signature_help,
        preferred_indices: help.completion.preferred_indices,
    }
}
