use std::marker::PhantomData;

use builtin_fn::ParamRef;

use crate::builtins::BuiltinKey;
use crate::core::columns::{AbiKind, Column, ColumnKind, KernelColumn, KernelResult};
use crate::core::errors::EvalError;
use crate::core::types::{EvalBlock, Mask};
use crate::ir::{
    DebugArgumentContract, DebugCallContract, PlanId, PlanOwner, PlannedArgument,
    PlannedArgumentKind,
};

use super::convert_column;
use super::debug::{assert_debug_inputs, assert_debug_output};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepeatGroups<G>(Box<[G]>);

impl<G> RepeatGroups<G> {
    pub(crate) fn new(groups: Vec<G>) -> Self {
        Self(groups.into_boxed_slice())
    }

    pub(crate) fn into_vec(self) -> Vec<G> {
        self.0.into_vec()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ValuePlan<K: ColumnKind> {
    owner: PlanOwner,
    id: PlanId,
    #[cfg(debug_assertions)]
    contract: Option<DebugArgumentContract>,
    kind: PhantomData<K>,
}

#[derive(Clone, Debug)]
pub(crate) struct ThunkPlan<K: ColumnKind> {
    owner: PlanOwner,
    body: PlanId,
    #[cfg(debug_assertions)]
    contract: Option<DebugArgumentContract>,
    kind: PhantomData<K>,
}

#[derive(Clone, Debug)]
pub(crate) struct LambdaPlan<K: ColumnKind> {
    owner: PlanOwner,
    body: PlanId,
    parameters: Box<[String]>,
    #[cfg(debug_assertions)]
    contract: Option<DebugArgumentContract>,
    kind: PhantomData<K>,
}

#[derive(Clone, Debug)]
pub(crate) struct BinderHandle<K: ColumnKind> {
    owner: PlanOwner,
    name: String,
    kind: PhantomData<K>,
}

impl<K: ColumnKind> BinderHandle<K> {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn owner(&self) -> PlanOwner {
        self.owner
    }
}

impl<K: ColumnKind> ValuePlan<K> {
    fn new(owner: PlanOwner, id: PlanId, contract: Option<DebugArgumentContract>) -> Self {
        #[cfg(not(debug_assertions))]
        let _ = contract;
        Self {
            owner,
            id,
            #[cfg(debug_assertions)]
            contract,
            kind: PhantomData,
        }
    }

    pub(crate) fn into_parts(self) -> (PlanOwner, PlanId, Option<DebugArgumentContract>) {
        (self.owner, self.id, {
            #[cfg(debug_assertions)]
            {
                self.contract
            }
            #[cfg(not(debug_assertions))]
            {
                None
            }
        })
    }
}

impl<K: ColumnKind> ThunkPlan<K> {
    fn new(owner: PlanOwner, body: PlanId, contract: Option<DebugArgumentContract>) -> Self {
        #[cfg(not(debug_assertions))]
        let _ = contract;
        Self {
            owner,
            body,
            #[cfg(debug_assertions)]
            contract,
            kind: PhantomData,
        }
    }

    pub(crate) fn into_parts(self) -> (PlanOwner, PlanId, Option<DebugArgumentContract>) {
        (self.owner, self.body, {
            #[cfg(debug_assertions)]
            {
                self.contract
            }
            #[cfg(not(debug_assertions))]
            {
                None
            }
        })
    }
}

impl<K: ColumnKind> LambdaPlan<K> {
    fn new(
        owner: PlanOwner,
        body: PlanId,
        parameters: Box<[String]>,
        contract: Option<DebugArgumentContract>,
    ) -> Self {
        #[cfg(not(debug_assertions))]
        let _ = contract;
        Self {
            owner,
            body,
            parameters,
            #[cfg(debug_assertions)]
            contract,
            kind: PhantomData,
        }
    }

    pub(crate) fn parameters(&self) -> &[String] {
        &self.parameters
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        PlanOwner,
        PlanId,
        Box<[String]>,
        Option<DebugArgumentContract>,
    ) {
        (self.owner, self.body, self.parameters, {
            #[cfg(debug_assertions)]
            {
                self.contract
            }
            #[cfg(not(debug_assertions))]
            {
                None
            }
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LambdaBindings(Box<[(String, Column)]>);

impl LambdaBindings {
    pub(crate) fn new(bindings: Vec<(String, Column)>) -> Self {
        Self(bindings.into_boxed_slice())
    }

    pub(crate) fn as_slice(&self) -> &[(String, Column)] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ArgumentKey {
    parameter: ParamRef,
    repeat_group: Option<usize>,
}

impl ArgumentKey {
    fn new(parameter: ParamRef, repeat_group: Option<usize>) -> Self {
        Self {
            parameter,
            repeat_group,
        }
    }

    fn missing_error(self) -> PreparedArgumentError {
        PreparedArgumentError::Missing {
            parameter: self.parameter,
            repeat_group: self.repeat_group,
        }
    }

    fn duplicate_error(self) -> PreparedArgumentError {
        PreparedArgumentError::Duplicate {
            parameter: self.parameter,
            repeat_group: self.repeat_group,
        }
    }
}

struct ArgumentSlot<T> {
    key: ArgumentKey,
    value: Option<T>,
}

pub(super) struct ArgumentPool<T> {
    arguments: Vec<ArgumentSlot<T>>,
}

impl<T> ArgumentPool<T> {
    fn new(arguments: impl IntoIterator<Item = (ArgumentKey, T)>) -> Self {
        Self {
            arguments: arguments
                .into_iter()
                .map(|(key, value)| ArgumentSlot {
                    key,
                    value: Some(value),
                })
                .collect(),
        }
    }

    fn repeat_group_count(&self) -> usize {
        self.arguments
            .iter()
            .filter_map(|argument| {
                argument.value.as_ref()?;
                argument.key.repeat_group
            })
            .max()
            .unwrap_or(0)
    }

    fn take(&mut self, key: ArgumentKey) -> Result<T, PreparedArgumentError> {
        self.take_optional(key)?.ok_or_else(|| key.missing_error())
    }

    fn take_optional(&mut self, key: ArgumentKey) -> Result<Option<T>, PreparedArgumentError> {
        let mut matches = self
            .arguments
            .iter()
            .enumerate()
            .filter_map(|(index, argument)| {
                argument.value.as_ref()?;
                (argument.key == key).then_some(index)
            });
        let first = matches.next();
        if matches.next().is_some() {
            return Err(key.duplicate_error());
        }
        Ok(first.and_then(|index| self.arguments[index].value.take()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PreparedArgumentError {
    Missing {
        parameter: ParamRef,
        repeat_group: Option<usize>,
    },
    Duplicate {
        parameter: ParamRef,
        repeat_group: Option<usize>,
    },
    WrongPhysicalKind {
        expected: AbiKind,
        actual: AbiKind,
    },
    WrongPlanKind,
}

#[derive(Clone, Debug)]
pub(crate) struct EvaluatedArgument {
    pub parameter: ParamRef,
    pub repeat_group: Option<usize>,
    pub block: EvalBlock,
}

pub(crate) struct PreparedValueArguments {
    arguments: ArgumentPool<EvalBlock>,
    eligible: Mask,
    upstream_ok: Mask,
    upstream_errors: Vec<(usize, EvalError)>,
    execution_mask: Mask,
}

impl PreparedValueArguments {
    pub(crate) fn new(
        arguments: Vec<EvaluatedArgument>,
        execution_mask: &Mask,
        key: BuiltinKey,
        debug_contract: Option<&DebugCallContract>,
    ) -> Self {
        let mut upstream_ok = Mask::all(execution_mask.len());
        let mut upstream_errors = Vec::new();
        for argument in &arguments {
            debug_assert_eq!(argument.block.len(), execution_mask.len());
            for index in 0..upstream_ok.len() {
                if !argument.block.ok[index] {
                    upstream_ok.set(index, false);
                }
            }
            upstream_errors.extend(argument.block.errors.iter().cloned());
        }
        let eligible = execution_mask.and(&upstream_ok);
        assert_debug_inputs(key, &arguments, execution_mask, debug_contract);
        Self {
            arguments: ArgumentPool::new(arguments.into_iter().map(|argument| {
                (
                    ArgumentKey::new(argument.parameter, argument.repeat_group),
                    argument.block,
                )
            })),
            eligible,
            upstream_ok,
            upstream_errors,
            execution_mask: execution_mask.clone(),
        }
    }

    pub(crate) fn eligible(&self) -> &Mask {
        &self.eligible
    }

    pub(crate) fn repeat_group_count(&self) -> usize {
        self.arguments.repeat_group_count()
    }

    pub(crate) fn take_value<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<KernelColumn<K>, PreparedArgumentError> {
        let block = self
            .arguments
            .take(ArgumentKey::new(parameter, repeat_group))?;
        convert_column::<K>(block.column)
    }

    pub(crate) fn take_optional_value<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<KernelColumn<K>>, PreparedArgumentError> {
        let Some(block) = self
            .arguments
            .take_optional(ArgumentKey::new(parameter, repeat_group))?
        else {
            return Ok(None);
        };
        convert_column::<K>(block.column).map(Some)
    }

    pub(crate) fn contract_failure(self, _error: PreparedArgumentError) -> EvalBlock {
        let mut failure = EvalBlock::fail_mask(&self.execution_mask, EvalError::TypeMismatch);
        failure.errors.extend(self.upstream_errors);
        failure
    }

    pub(crate) fn invalid_mode(self, _key: BuiltinKey) -> EvalBlock {
        self.contract_failure(PreparedArgumentError::WrongPlanKind)
    }

    pub(crate) fn finish<K: ColumnKind>(
        self,
        result: KernelResult<K>,
        key: BuiltinKey,
        debug_contract: Option<&DebugCallContract>,
    ) -> EvalBlock {
        let mut block = result.into_eval_block();
        for row in 0..block.len() {
            if !self.upstream_ok[row] {
                block.ok.set(row, false);
            }
        }
        block.errors.extend(self.upstream_errors);
        assert_debug_output(
            key,
            &block,
            &self.execution_mask,
            &self.eligible,
            debug_contract,
        );
        block
    }
}

pub(crate) struct PreparedControlledArguments {
    owner: PlanOwner,
    arguments: ArgumentPool<PlannedArgumentKind>,
    #[cfg(debug_assertions)]
    contracts: Box<[DebugArgumentContract]>,
}

impl PreparedControlledArguments {
    pub(crate) fn new(
        owner: PlanOwner,
        arguments: &[PlannedArgument],
        debug_contract: Option<&DebugCallContract>,
    ) -> Self {
        #[cfg(not(debug_assertions))]
        let _ = debug_contract;
        Self {
            owner,
            arguments: ArgumentPool::new(arguments.iter().map(|argument| {
                (
                    ArgumentKey::new(argument.parameter, argument.repeat_group),
                    argument.kind.clone(),
                )
            })),
            #[cfg(debug_assertions)]
            contracts: debug_contract
                .map(|contract| contract.arguments.clone())
                .unwrap_or_default(),
        }
    }

    pub(crate) fn repeat_group_count(&self) -> usize {
        self.arguments.repeat_group_count()
    }

    pub(crate) fn take_value<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<ValuePlan<K>, PreparedArgumentError> {
        let key = ArgumentKey::new(parameter, repeat_group);
        match self.arguments.take(key)? {
            PlannedArgumentKind::Value(id) => {
                Ok(ValuePlan::new(self.owner, id, self.contract(key)))
            }
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    // These explicit decoders are part of the complete generated ABI even though the
    // current controlled catalog has no optional parameters of these four shapes.
    #[allow(dead_code)]
    pub(crate) fn take_optional_value<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<ValuePlan<K>>, PreparedArgumentError> {
        let key = ArgumentKey::new(parameter, repeat_group);
        let Some(argument) = self.arguments.take_optional(key)? else {
            return Ok(None);
        };
        match argument {
            PlannedArgumentKind::Value(id) => {
                Ok(Some(ValuePlan::new(self.owner, id, self.contract(key))))
            }
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    pub(crate) fn take_thunk<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<ThunkPlan<K>, PreparedArgumentError> {
        let key = ArgumentKey::new(parameter, repeat_group);
        match self.arguments.take(key)? {
            PlannedArgumentKind::Thunk { body } => {
                Ok(ThunkPlan::new(self.owner, body, self.contract(key)))
            }
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn take_optional_thunk<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<ThunkPlan<K>>, PreparedArgumentError> {
        let key = ArgumentKey::new(parameter, repeat_group);
        let Some(argument) = self.arguments.take_optional(key)? else {
            return Ok(None);
        };
        match argument {
            PlannedArgumentKind::Thunk { body } => {
                Ok(Some(ThunkPlan::new(self.owner, body, self.contract(key))))
            }
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    pub(crate) fn take_lambda<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<LambdaPlan<K>, PreparedArgumentError> {
        let key = ArgumentKey::new(parameter, repeat_group);
        match self.arguments.take(key)? {
            PlannedArgumentKind::Lambda { body, parameters } => Ok(LambdaPlan::new(
                self.owner,
                body,
                parameters,
                self.contract(key),
            )),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn take_optional_lambda<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<LambdaPlan<K>>, PreparedArgumentError> {
        let key = ArgumentKey::new(parameter, repeat_group);
        let Some(argument) = self.arguments.take_optional(key)? else {
            return Ok(None);
        };
        match argument {
            PlannedArgumentKind::Lambda { body, parameters } => Ok(Some(LambdaPlan::new(
                self.owner,
                body,
                parameters,
                self.contract(key),
            ))),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    pub(crate) fn take_binder<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<BinderHandle<K>, PreparedArgumentError> {
        let key = ArgumentKey::new(parameter, repeat_group);
        match self.arguments.take(key)? {
            PlannedArgumentKind::Binder { name } => Ok(BinderHandle {
                owner: self.owner,
                name,
                kind: PhantomData,
            }),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn take_optional_binder<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<BinderHandle<K>>, PreparedArgumentError> {
        let key = ArgumentKey::new(parameter, repeat_group);
        let Some(argument) = self.arguments.take_optional(key)? else {
            return Ok(None);
        };
        match argument {
            PlannedArgumentKind::Binder { name } => Ok(Some(BinderHandle {
                owner: self.owner,
                name,
                kind: PhantomData,
            })),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    fn contract(&self, key: ArgumentKey) -> Option<DebugArgumentContract> {
        #[cfg(debug_assertions)]
        {
            self.contracts
                .iter()
                .find(|contract| ArgumentKey::new(contract.parameter, contract.repeat_group) == key)
                .cloned()
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = key;
            None
        }
    }

    pub(crate) fn contract_failure(self, _error: PreparedArgumentError, mask: &Mask) -> EvalBlock {
        EvalBlock::fail_mask(mask, EvalError::TypeMismatch)
    }

    pub(crate) fn invalid_mode(self, _key: BuiltinKey, mask: &Mask) -> EvalBlock {
        self.contract_failure(PreparedArgumentError::WrongPlanKind, mask)
    }
}

#[cfg(test)]
mod tests {
    use super::{ArgumentKey, ArgumentPool, PreparedArgumentError};
    use builtin_fn::ParamRef;

    #[test]
    fn argument_pool_locates_and_destructively_takes_arguments() {
        let head = ArgumentKey::new(ParamRef::Head(0), None);
        let repeat_one = ArgumentKey::new(ParamRef::Repeat(0), Some(1));
        let repeat_two = ArgumentKey::new(ParamRef::Repeat(0), Some(2));
        let missing = ArgumentKey::new(ParamRef::Tail(0), None);
        let mut pool = ArgumentPool::new([
            (head, "head"),
            (repeat_one, "repeat one"),
            (repeat_two, "repeat two"),
        ]);

        assert_eq!(pool.repeat_group_count(), 2);
        assert_eq!(pool.take(head), Ok("head"));
        assert_eq!(pool.take_optional(head), Ok(None));
        assert_eq!(pool.take_optional(missing), Ok(None));
        assert_eq!(
            pool.take(missing),
            Err(PreparedArgumentError::Missing {
                parameter: ParamRef::Tail(0),
                repeat_group: None,
            })
        );
        assert_eq!(pool.take(repeat_two), Ok("repeat two"));
        assert_eq!(pool.repeat_group_count(), 1);
    }

    #[test]
    fn argument_pool_reports_duplicate_keys_before_consuming_them() {
        let key = ArgumentKey::new(ParamRef::Repeat(1), Some(2));
        let mut pool = ArgumentPool::new([(key, "first"), (key, "second")]);

        assert_eq!(
            pool.take_optional(key),
            Err(PreparedArgumentError::Duplicate {
                parameter: ParamRef::Repeat(1),
                repeat_group: Some(2),
            })
        );
        assert_eq!(
            pool.take(key),
            Err(PreparedArgumentError::Duplicate {
                parameter: ParamRef::Repeat(1),
                repeat_group: Some(2),
            })
        );
    }
}
