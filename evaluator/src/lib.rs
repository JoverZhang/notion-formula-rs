pub mod core;
mod ir;
mod kernels;
mod planner;
mod runtime;

pub use core::context::EvalContext;
pub use core::errors::{EvalError, ProviderError, SimpleEvalError};
pub use core::provider::Provider;
pub use core::types::{Column, ColumnBlock, EvalBlock, Mask, NullMap, RowBatch, RowId, Value};
pub use runtime::evaluator::Evaluator;

#[cfg(test)]
mod tests;
