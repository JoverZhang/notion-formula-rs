//! Synchronous row-batch formula evaluator.

mod builtins;
pub mod core;
mod ir;
mod kernels;
mod planner;
mod runtime;

pub use core::columns::{
    AbiKind, AnyKind, BooleanKind, Column, ColumnKind, DateKind, KernelColumn, KernelResult,
    ListKind, NumberKind, SharedBitmap, SharedStorage, TextKind, Validity,
};
pub use core::context::{BuiltinKernelContext, BuiltinRuntimeContext, EvalContext};
pub use core::errors::{EvalError, InputContractError, PrepareError};
pub use core::inputs::{EvalInputs, EvalInputsBuilder, InputSlot, RequiredColumn};
pub use core::types::{EvalBlock, Mask, RowBatch, RowId, Value};
pub use planner::{PreparedFormula, prepare_formula};

#[cfg(test)]
mod tests;
