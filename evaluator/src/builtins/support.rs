use std::sync::LazyLock;

use builtin_fn::FunctionSig;

use crate::builtins::BuiltinKey;
use crate::core::columns::{BooleanKind, Column, ColumnKind, KernelColumn, KernelResult, Validity};
use crate::core::errors::EvalError;
use crate::core::types::{EvalBlock, Mask, Value};
use crate::ir::{DebugCallContract, PlanOwner};

mod arguments;
mod debug;

pub(crate) use arguments::{
    BinderHandle, EvaluatedArgument, LambdaBindings, LambdaPlan, PreparedArgumentError,
    PreparedControlledArguments, PreparedValueArguments, RepeatGroups, ThunkPlan, ValuePlan,
};
pub(crate) use debug::{assert_debug_output, assert_lambda_bindings, assert_materialized_argument};

pub(crate) trait BuiltinEvalContext {
    fn plan_owner(&self) -> PlanOwner;

    fn eval<K: ColumnKind>(&mut self, plan: ValuePlan<K>, mask: &Mask) -> KernelResult<K>;

    fn eval_thunk<K: ColumnKind>(&mut self, plan: ThunkPlan<K>, mask: &Mask) -> KernelResult<K>;

    fn apply_lambda<K: ColumnKind>(
        &mut self,
        plan: LambdaPlan<K>,
        bindings: LambdaBindings,
        mask: &Mask,
    ) -> KernelResult<K>;

    fn split_mask(&self, condition: &KernelResult<BooleanKind>, parent: &Mask) -> ConditionSplit {
        debug_assert_eq!(condition.column.len(), parent.len());
        debug_assert_eq!(condition.ok.len(), parent.len());
        let mut when_true = Mask::none(parent.len());
        let mut when_false = Mask::none(parent.len());
        for row in 0..parent.len() {
            if !parent[row] || !condition.ok[row] {
                continue;
            }
            match condition.column.value(row) {
                Some(true) => when_true.set(row, true),
                Some(false) | None => when_false.set(row, true),
            }
        }
        ConditionSplit {
            when_true,
            when_false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConditionSplit {
    pub(crate) when_true: Mask,
    pub(crate) when_false: Mask,
}

pub(crate) fn finish_controlled_result<K: ColumnKind>(
    result: KernelResult<K>,
    key: BuiltinKey,
    mask: &Mask,
    debug_contract: Option<&DebugCallContract>,
) -> EvalBlock {
    let block = result.into_eval_block();
    assert_debug_output(key, &block, mask, mask, debug_contract);
    block
}

pub(crate) fn convert_column<K: ColumnKind>(
    column: Column,
) -> Result<KernelColumn<K>, PreparedArgumentError> {
    K::from_column(column).map_err(|column| {
        let actual = column.abi_kind();
        debug_assert_eq!(
            actual,
            K::ABI_KIND,
            "typed builtin argument expected physical ABI {:?}, observed {:?}",
            K::ABI_KIND,
            actual
        );
        PreparedArgumentError::WrongPhysicalKind {
            expected: K::ABI_KIND,
            actual,
        }
    })
}

pub(crate) fn block_into_kernel<K: ColumnKind>(block: EvalBlock, mask: &Mask) -> KernelResult<K> {
    match convert_column::<K>(block.column) {
        Ok(column) => KernelResult {
            column,
            ok: block.ok,
            errors: block.errors,
        },
        Err(_) => {
            let failure = EvalBlock::fail_mask(mask, EvalError::TypeMismatch);
            KernelResult {
                column: KernelColumn::from_values(
                    vec![K::placeholder(); mask.len()],
                    Validity::AllValid,
                ),
                ok: failure.ok,
                errors: failure.errors,
            }
        }
    }
}

pub(crate) fn rows_to_kernel<K: ColumnKind>(rows: Vec<RowOutcome>, mask: &Mask) -> KernelResult<K> {
    debug_assert_eq!(rows.len(), mask.len());
    let mut values = Vec::with_capacity(rows.len());
    let mut valid = Vec::with_capacity(rows.len());
    let mut ok = Mask::all(rows.len());
    let mut errors = Vec::new();

    for (index, outcome) in rows.into_iter().enumerate() {
        match outcome {
            RowOutcome::Value(value) => match K::from_value(value) {
                Ok(value) => {
                    values.push(value);
                    valid.push(true);
                }
                Err(_) => {
                    values.push(K::placeholder());
                    valid.push(true);
                    ok.set(index, false);
                    errors.push((index, EvalError::TypeMismatch));
                }
            },
            RowOutcome::Null => {
                values.push(K::placeholder());
                valid.push(false);
            }
            RowOutcome::Inactive => {
                values.push(K::placeholder());
                valid.push(true);
            }
            RowOutcome::Failed => {
                values.push(K::placeholder());
                valid.push(true);
                ok.set(index, false);
            }
            RowOutcome::Error(error) => {
                values.push(K::placeholder());
                valid.push(true);
                ok.set(index, false);
                errors.push((index, error));
            }
        }
    }

    KernelResult {
        column: KernelColumn::from_values(values, Validity::from_valid_bits(valid)),
        ok,
        errors,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RowOutcome {
    Value(Value),
    Null,
    Inactive,
    Failed,
    Error(EvalError),
}

pub(crate) fn signature_for_key(key: BuiltinKey) -> &'static FunctionSig {
    static SIGNATURES: LazyLock<Vec<FunctionSig>> = LazyLock::new(builtin_fn::builtins_functions);
    SIGNATURES
        .iter()
        .find(|signature| signature.name == key.name())
        .expect("generated builtin key must have a catalog signature")
}
