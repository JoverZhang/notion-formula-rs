use std::collections::HashMap;

use analyzer::analysis::{Property, Ty};

use super::types::{RowBatch, RowId};

#[derive(Clone, Debug)]
pub struct EvalContext {
    pub properties: Vec<Property>,
    prop_index: HashMap<String, usize>,
}

impl EvalContext {
    pub fn new(properties: Vec<Property>) -> Self {
        let mut prop_index = HashMap::with_capacity(properties.len());
        for (index, property) in properties.iter().enumerate() {
            prop_index.entry(property.name.clone()).or_insert(index);
        }
        Self {
            properties,
            prop_index,
        }
    }

    pub fn property(&self, name: &str) -> Option<&Property> {
        self.prop_index
            .get(name)
            .and_then(|index| self.properties.get(*index))
    }

    pub fn ty(&self, name: &str) -> Option<&Ty> {
        self.property(name).map(|property| &property.ty)
    }
}

/// Immutable system-data snapshot shared by every kernel in one evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltinRuntimeContext {
    evaluated_at_epoch_ms: i64,
    timezone_offset_minutes: i32,
}

impl BuiltinRuntimeContext {
    pub fn new(evaluated_at_epoch_ms: i64, timezone_offset_minutes: i32) -> Self {
        Self {
            evaluated_at_epoch_ms,
            timezone_offset_minutes,
        }
    }

    pub fn evaluated_at_epoch_ms(&self) -> i64 {
        self.evaluated_at_epoch_ms
    }

    pub fn timezone_offset_minutes(&self) -> i32 {
        self.timezone_offset_minutes
    }
}

/// Concrete, read-only system data visible to builtin kernels for one batch.
///
/// The caller-owned runtime snapshot and the current row identities deliberately remain
/// separate inputs. `now()`/`today()` read the frozen snapshot, while `id()` reads the
/// corresponding row from the batch.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinKernelContext<'a> {
    runtime: &'a BuiltinRuntimeContext,
    batch: &'a RowBatch,
}

/// Read-only system data needed by eager builtin kernels.
///
/// Generated kernel contracts depend on this interface so a concrete runtime may keep
/// borrowed storage without leaking its lifetime into the generated trait ABI.
pub(crate) trait BuiltinValueContext {
    fn runtime(&self) -> &BuiltinRuntimeContext;

    fn rows(&self) -> &[RowId];
}

impl<'a> BuiltinKernelContext<'a> {
    pub(crate) fn new(runtime: &'a BuiltinRuntimeContext, batch: &'a RowBatch) -> Self {
        Self { runtime, batch }
    }

    pub fn runtime(&self) -> &BuiltinRuntimeContext {
        self.runtime
    }

    pub fn rows(&self) -> &[RowId] {
        self.batch.rows()
    }

    pub fn batch(&self) -> &RowBatch {
        self.batch
    }
}

impl BuiltinValueContext for BuiltinKernelContext<'_> {
    fn runtime(&self) -> &BuiltinRuntimeContext {
        self.runtime
    }

    fn rows(&self) -> &[RowId] {
        self.batch.rows()
    }
}
