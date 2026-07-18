//! Structured builtin catalog emitted by category declarations.

use crate::{FunctionCategory, FunctionSig};

/// All declarations belonging to one builtin category, in source order.
#[derive(Debug, Clone)]
pub struct BuiltinCategory {
    pub category: FunctionCategory,
    pub entries: Vec<BuiltinCatalogEntry>,
}

impl BuiltinCategory {
    pub fn new(category: FunctionCategory, entries: Vec<BuiltinCatalogEntry>) -> Self {
        debug_assert!(entries.iter().all(|entry| entry.category == category));
        Self { category, entries }
    }

    /// Consume this category and yield only executable signatures.
    pub fn into_functions(self) -> impl Iterator<Item = FunctionSig> {
        self.entries
            .into_iter()
            .filter_map(|entry| entry.implementation)
    }
}

/// Presentation and implementation metadata for one declaration.
#[derive(Debug, Clone)]
pub struct BuiltinCatalogEntry {
    pub name: String,
    pub signature: String,
    pub detail: String,
    pub docs: Vec<String>,
    pub category: FunctionCategory,
    /// `None` only for an `#[unsupported]` declaration.
    pub implementation: Option<FunctionSig>,
}

impl BuiltinCatalogEntry {
    #[doc(hidden)]
    pub fn supported(
        category: FunctionCategory,
        signature: impl Into<String>,
        detail: impl Into<String>,
        docs: Vec<String>,
        implementation: FunctionSig,
    ) -> Self {
        let name = implementation.name.clone();
        Self {
            name,
            signature: signature.into(),
            detail: detail.into(),
            docs,
            category,
            implementation: Some(implementation),
        }
    }

    #[doc(hidden)]
    pub fn unsupported(
        category: FunctionCategory,
        name: impl Into<String>,
        signature: impl Into<String>,
        detail: impl Into<String>,
        docs: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            signature: signature.into(),
            detail: detail.into(),
            docs,
            category,
            implementation: None,
        }
    }

    pub fn is_supported(&self) -> bool {
        self.implementation.is_some()
    }
}
