mod error;
mod grammar;
mod lexer;

use crate::{FunctionCategory, GenericParamKind};
use std::collections::HashMap;

pub use error::{BuiltinSigParseError, BuiltinSigParseErrorKind};

#[derive(Debug, Clone, Default)]
pub struct GenericKindRegistry {
    kinds: HashMap<String, GenericParamKind>,
}

impl GenericKindRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_kinds() -> Self {
        let mut registry = Self::new();
        registry.register("Plain", GenericParamKind::Plain);
        registry.register("Variant", GenericParamKind::Variant);
        registry
    }

    pub fn register(&mut self, name: impl Into<String>, kind: GenericParamKind) {
        self.kinds.insert(name.into(), kind);
    }

    pub fn resolve(&self, name: &str) -> Option<GenericParamKind> {
        self.kinds.get(name).copied()
    }
}

#[derive(Debug, Clone)]
pub struct BuiltinSigParser {
    registry: GenericKindRegistry,
}

impl BuiltinSigParser {
    pub fn new(registry: GenericKindRegistry) -> Self {
        Self { registry }
    }

    pub fn parse(
        &self,
        category: FunctionCategory,
        text: &str,
    ) -> Result<crate::FunctionSig, BuiltinSigParseError> {
        grammar::parse_signature(category, text, &self.registry)
    }
}
