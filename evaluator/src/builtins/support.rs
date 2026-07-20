use std::marker::PhantomData;
use std::sync::LazyLock;

#[cfg(debug_assertions)]
use analyzer::analysis::Ty;
use builtin_fn::{FunctionSig, ParamRef};
#[cfg(debug_assertions)]
use builtin_fn::{normalize_union, type_accepts};

use crate::builtins::BuiltinKey;
use crate::core::columns::{
    AbiKind, BooleanKind, Column, ColumnKind, KernelColumn, KernelResult, Validity,
};
use crate::core::errors::EvalError;
use crate::core::types::{EvalBlock, Mask, Value};
use crate::ir::{
    DebugArgumentContract, DebugCallContract, PlanId, PlanOwner, PlannedArgument,
    PlannedArgumentKind,
};

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

#[derive(Clone, Debug)]
pub(crate) struct DynamicValueArgs {
    head: Box<[Option<Column>]>,
    repeat_groups: Box<[Box<[Option<Column>]>]>,
    #[allow(dead_code)]
    tail: Box<[Option<Column>]>,
}

impl DynamicValueArgs {
    pub(crate) fn new(
        head: Vec<Option<Column>>,
        repeat_groups: Vec<Vec<Option<Column>>>,
        tail: Vec<Option<Column>>,
    ) -> Self {
        Self {
            head: head.into_boxed_slice(),
            repeat_groups: repeat_groups
                .into_iter()
                .map(Vec::into_boxed_slice)
                .collect(),
            tail: tail.into_boxed_slice(),
        }
    }

    pub(crate) fn head(&self, index: usize) -> Option<&Column> {
        self.head.get(index).and_then(Option::as_ref)
    }

    #[allow(dead_code)]
    pub(crate) fn tail(&self, index: usize) -> Option<&Column> {
        self.tail.get(index).and_then(Option::as_ref)
    }

    pub(crate) fn repeat_groups(&self) -> &[Box<[Option<Column>]>] {
        &self.repeat_groups
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
    arguments: Vec<Option<EvaluatedArgument>>,
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
            arguments: arguments.into_iter().map(Some).collect(),
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
        self.arguments
            .iter()
            .filter_map(|argument| argument.as_ref()?.repeat_group)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn take_value<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<KernelColumn<K>, PreparedArgumentError> {
        let index = self.find(parameter, repeat_group)?;
        let argument = self.arguments[index]
            .take()
            .expect("located argument exists");
        convert_column::<K>(argument.block.column)
    }

    #[allow(dead_code)]
    pub(crate) fn take_optional_value<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<KernelColumn<K>>, PreparedArgumentError> {
        let Some(index) = self.find_optional(parameter, repeat_group)? else {
            return Ok(None);
        };
        let argument = self.arguments[index]
            .take()
            .expect("located argument exists");
        convert_column::<K>(argument.block.column).map(Some)
    }

    fn find(
        &self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<usize, PreparedArgumentError> {
        self.find_optional(parameter, repeat_group)?
            .ok_or(PreparedArgumentError::Missing {
                parameter,
                repeat_group,
            })
    }

    fn find_optional(
        &self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<usize>, PreparedArgumentError> {
        let mut matches = self
            .arguments
            .iter()
            .enumerate()
            .filter_map(|(index, argument)| {
                let argument = argument.as_ref()?;
                (argument.parameter == parameter && argument.repeat_group == repeat_group)
                    .then_some(index)
            });
        let first = matches.next();
        if matches.next().is_some() {
            return Err(PreparedArgumentError::Duplicate {
                parameter,
                repeat_group,
            });
        }
        Ok(first)
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
    arguments: Vec<Option<PlannedArgument>>,
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
            arguments: arguments.iter().cloned().map(Some).collect(),
            #[cfg(debug_assertions)]
            contracts: debug_contract
                .map(|contract| contract.arguments.clone())
                .unwrap_or_default(),
        }
    }

    pub(crate) fn repeat_group_count(&self) -> usize {
        self.arguments
            .iter()
            .filter_map(|argument| argument.as_ref()?.repeat_group)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn take_value<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<ValuePlan<K>, PreparedArgumentError> {
        let argument = self.take(parameter, repeat_group)?;
        match argument.kind {
            PlannedArgumentKind::Value(id) => Ok(ValuePlan::new(
                self.owner,
                id,
                self.contract(parameter, repeat_group),
            )),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn take_optional_value<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<ValuePlan<K>>, PreparedArgumentError> {
        let Some(argument) = self.take_optional(parameter, repeat_group)? else {
            return Ok(None);
        };
        match argument.kind {
            PlannedArgumentKind::Value(id) => Ok(Some(ValuePlan::new(
                self.owner,
                id,
                self.contract(parameter, repeat_group),
            ))),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    pub(crate) fn take_thunk<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<ThunkPlan<K>, PreparedArgumentError> {
        let argument = self.take(parameter, repeat_group)?;
        match argument.kind {
            PlannedArgumentKind::Thunk { body } => Ok(ThunkPlan::new(
                self.owner,
                body,
                self.contract(parameter, repeat_group),
            )),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn take_optional_thunk<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<ThunkPlan<K>>, PreparedArgumentError> {
        let Some(argument) = self.take_optional(parameter, repeat_group)? else {
            return Ok(None);
        };
        match argument.kind {
            PlannedArgumentKind::Thunk { body } => Ok(Some(ThunkPlan::new(
                self.owner,
                body,
                self.contract(parameter, repeat_group),
            ))),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    pub(crate) fn take_lambda<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<LambdaPlan<K>, PreparedArgumentError> {
        let argument = self.take(parameter, repeat_group)?;
        match argument.kind {
            PlannedArgumentKind::Lambda { body, parameters } => Ok(LambdaPlan::new(
                self.owner,
                body,
                parameters,
                self.contract(parameter, repeat_group),
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
        let Some(argument) = self.take_optional(parameter, repeat_group)? else {
            return Ok(None);
        };
        match argument.kind {
            PlannedArgumentKind::Lambda { body, parameters } => Ok(Some(LambdaPlan::new(
                self.owner,
                body,
                parameters,
                self.contract(parameter, repeat_group),
            ))),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    pub(crate) fn take_binder<K: ColumnKind>(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<BinderHandle<K>, PreparedArgumentError> {
        let argument = self.take(parameter, repeat_group)?;
        match argument.kind {
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
        let Some(argument) = self.take_optional(parameter, repeat_group)? else {
            return Ok(None);
        };
        match argument.kind {
            PlannedArgumentKind::Binder { name } => Ok(Some(BinderHandle {
                owner: self.owner,
                name,
                kind: PhantomData,
            })),
            _ => Err(PreparedArgumentError::WrongPlanKind),
        }
    }

    fn take(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<PlannedArgument, PreparedArgumentError> {
        self.take_optional(parameter, repeat_group)?
            .ok_or(PreparedArgumentError::Missing {
                parameter,
                repeat_group,
            })
    }

    fn take_optional(
        &mut self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Result<Option<PlannedArgument>, PreparedArgumentError> {
        let mut matches = self
            .arguments
            .iter()
            .enumerate()
            .filter_map(|(index, argument)| {
                let argument = argument.as_ref()?;
                (argument.parameter == parameter && argument.repeat_group == repeat_group)
                    .then_some(index)
            });
        let first = matches.next();
        if matches.next().is_some() {
            return Err(PreparedArgumentError::Duplicate {
                parameter,
                repeat_group,
            });
        }
        Ok(first.and_then(|index| self.arguments[index].take()))
    }

    fn contract(
        &self,
        parameter: ParamRef,
        repeat_group: Option<usize>,
    ) -> Option<DebugArgumentContract> {
        #[cfg(debug_assertions)]
        {
            self.contracts
                .iter()
                .find(|contract| {
                    contract.parameter == parameter && contract.repeat_group == repeat_group
                })
                .cloned()
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = (parameter, repeat_group);
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

#[cfg(debug_assertions)]
pub(crate) fn assert_materialized_argument(
    contract: Option<&DebugArgumentContract>,
    block: &EvalBlock,
    mask: &Mask,
) {
    let Some(contract) = contract else {
        return;
    };
    assert_runtime_type_rows(
        &format!(
            "builtin argument {:?} group {:?}",
            contract.parameter, contract.repeat_group
        ),
        block,
        mask,
        &contract.expected_ty,
    );
}

#[cfg(not(debug_assertions))]
pub(crate) fn assert_materialized_argument(
    _contract: Option<&DebugArgumentContract>,
    _block: &EvalBlock,
    _mask: &Mask,
) {
}

#[cfg(debug_assertions)]
pub(crate) fn assert_lambda_bindings(
    contract: Option<&DebugArgumentContract>,
    bindings: &LambdaBindings,
    mask: &Mask,
) {
    let Some(contract) = contract else {
        return;
    };
    let Ty::Fn { params, .. } = &contract.expected_ty else {
        panic!(
            "builtin argument {:?} group {:?} expected a lambda contract, observed {}",
            contract.parameter, contract.repeat_group, contract.expected_ty
        );
    };
    assert_eq!(
        params.len(),
        bindings.as_slice().len(),
        "builtin lambda binding count for argument {:?} group {:?}",
        contract.parameter,
        contract.repeat_group
    );
    for ((_, expected_ty), (name, column)) in params.iter().zip(bindings.as_slice()) {
        let block = EvalBlock::new(column.clone(), Mask::all(mask.len()), Vec::new());
        assert_runtime_type_rows(
            &format!(
                "builtin lambda binding {name} for argument {:?} group {:?}",
                contract.parameter, contract.repeat_group
            ),
            &block,
            mask,
            expected_ty,
        );
    }
}

#[cfg(not(debug_assertions))]
pub(crate) fn assert_lambda_bindings(
    _contract: Option<&DebugArgumentContract>,
    _bindings: &LambdaBindings,
    _mask: &Mask,
) {
}

#[cfg(debug_assertions)]
fn assert_debug_inputs(
    key: BuiltinKey,
    arguments: &[EvaluatedArgument],
    execution_mask: &Mask,
    contract: Option<&DebugCallContract>,
) {
    let Some(contract) = contract else {
        return;
    };
    for argument in arguments {
        let expected = contract.arguments.iter().find(|expected| {
            expected.parameter == argument.parameter
                && expected.repeat_group == argument.repeat_group
        });
        let Some(expected) = expected else {
            panic!(
                "builtin {} has no resolved contract for {:?} group {:?}",
                key.name(),
                argument.parameter,
                argument.repeat_group
            );
        };
        assert_runtime_type_rows(
            &format!(
                "builtin {} argument {:?} group {:?}",
                key.name(),
                argument.parameter,
                argument.repeat_group
            ),
            &argument.block,
            execution_mask,
            &expected.expected_ty,
        );
    }
}

#[cfg(not(debug_assertions))]
fn assert_debug_inputs(
    _key: BuiltinKey,
    _arguments: &[EvaluatedArgument],
    _execution_mask: &Mask,
    _contract: Option<&DebugCallContract>,
) {
}

#[cfg(debug_assertions)]
pub(crate) fn assert_debug_output(
    key: BuiltinKey,
    block: &EvalBlock,
    execution_mask: &Mask,
    type_check_mask: &Mask,
    contract: Option<&DebugCallContract>,
) {
    assert_eq!(
        block.len(),
        execution_mask.len(),
        "builtin {} output length",
        key.name()
    );
    assert_eq!(
        block.ok.len(),
        execution_mask.len(),
        "builtin {} ok length",
        key.name()
    );
    assert_eq!(type_check_mask.len(), execution_mask.len());
    if let Some(length) = block.validity().bitmap_len() {
        assert_eq!(
            length,
            execution_mask.len(),
            "builtin {} validity length",
            key.name()
        );
    }
    for (row, _) in &block.errors {
        assert!(
            *row < execution_mask.len(),
            "builtin {} error row out of bounds",
            key.name()
        );
        assert!(
            execution_mask[*row],
            "builtin {} error row {} was not executed",
            key.name(),
            row
        );
        assert!(
            !block.ok[*row],
            "builtin {} error row remains ok",
            key.name()
        );
    }
    for row in 0..execution_mask.len() {
        let has_error = block.errors.iter().any(|(error_row, _)| *error_row == row);
        assert!(
            block.ok[row] || has_error,
            "builtin {} failed row {} has no error",
            key.name(),
            row
        );
        if !execution_mask[row] {
            assert!(
                block.ok[row],
                "builtin {} inactive row {} failed",
                key.name(),
                row
            );
            assert!(
                block.validity().is_valid(row),
                "builtin {} inactive row {} became null",
                key.name(),
                row
            );
        }
    }
    let Some(contract) = contract else {
        return;
    };
    assert_runtime_type_rows(
        &format!("builtin {} return", key.name()),
        block,
        type_check_mask,
        &contract.return_ty,
    );
}

#[cfg(not(debug_assertions))]
pub(crate) fn assert_debug_output(
    _key: BuiltinKey,
    _block: &EvalBlock,
    _execution_mask: &Mask,
    _type_check_mask: &Mask,
    _contract: Option<&DebugCallContract>,
) {
}

#[cfg(debug_assertions)]
pub(crate) fn assert_runtime_type_rows(
    context: &str,
    block: &EvalBlock,
    mask: &Mask,
    expected: &Ty,
) {
    assert_eq!(block.len(), mask.len(), "{context} mask length");
    for row in 0..block.len() {
        if !mask[row] || !block.ok[row] || !block.validity().is_valid(row) {
            continue;
        }
        let value = block.column.row_value(row).expect("valid row");
        let actual = runtime_ty(&value);
        assert!(
            type_accepts(expected, &actual),
            "{context} row {row} expected {expected}, observed {actual}"
        );
    }
}

#[cfg(debug_assertions)]
pub(crate) fn runtime_ty(value: &Value) -> Ty {
    match value {
        Value::Number(_) => Ty::Number,
        Value::Text(_) => Ty::String,
        Value::Bool(_) => Ty::Boolean,
        Value::Date(_) => Ty::Date,
        Value::List(values) if values.is_empty() => Ty::List(Box::new(Ty::Unknown)),
        Value::List(values) => Ty::List(Box::new(normalize_union(
            values.iter().map(runtime_ty).collect::<Vec<_>>(),
        ))),
    }
}
