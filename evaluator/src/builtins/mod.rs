mod implementations;
mod support;

pub(crate) mod contract {
    use builtin_fn::{FunctionSig, ParamRef};

    use crate::core::columns::{
        AbiKind, AnyKind, BooleanKind, DateKind, KernelColumn, KernelResult, ListKind, NumberKind,
        TextKind,
    };
    use crate::core::context::BuiltinValueContext;
    use crate::core::types::{EvalBlock, Mask};
    use crate::ir::DebugCallContract;

    use super::support::{
        BinderHandle, BuiltinEvalContext, DynamicValueArgs, LambdaPlan, PreparedArgumentError,
        PreparedControlledArguments, PreparedValueArguments, RepeatGroups, ThunkPlan, ValuePlan,
        finish_controlled_result, signature_for_key,
    };

    include!(concat!(env!("OUT_DIR"), "/builtin_contract.rs"));
}

pub(crate) use contract::{BuiltinEvaluationMode, BuiltinKey, dispatch_controlled, dispatch_value};
pub(crate) use support::*;
