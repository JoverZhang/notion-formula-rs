use std::collections::HashMap;

use crate::builtins::{
    BuiltinEvalContext, BuiltinEvaluationMode, EvaluatedArgument, LambdaBindings, LambdaPlan,
    PreparedControlledArguments, PreparedValueArguments, ThunkPlan, ValuePlan,
    assert_materialized_argument, block_into_kernel, dispatch_controlled, dispatch_value,
};
use crate::core::columns::{Column, ColumnKind, KernelResult};
use crate::core::context::BuiltinKernelContext;
use crate::core::errors::EvalError;
use crate::core::inputs::EvalInputs;
use crate::core::types::{EvalBlock, Mask, RowBatch};
use crate::ir::{ExecNode, ExecPlan, PlanId, PlannedArgumentKind};

use super::operators::{
    eval_binary, eval_cast, eval_list, eval_logical_and, eval_logical_or, eval_unary, literal_block,
};

pub(crate) struct Runtime<'a> {
    plan: &'a ExecPlan,
    batch: &'a RowBatch,
    inputs: &'a EvalInputs,
    scopes: Vec<HashMap<String, Column>>,
}

impl<'a> Runtime<'a> {
    pub(crate) fn new(plan: &'a ExecPlan, batch: &'a RowBatch, inputs: &'a EvalInputs) -> Self {
        Self {
            plan,
            batch,
            inputs,
            scopes: Vec::new(),
        }
    }

    pub(crate) fn evaluate(mut self, mask: &Mask) -> EvalBlock {
        self.eval_node(self.plan.root(), mask)
    }

    fn eval_node(&mut self, id: PlanId, mask: &Mask) -> EvalBlock {
        debug_assert_eq!(mask.len(), self.batch.len());
        match self.plan.node(id).clone() {
            ExecNode::Literal(value) => literal_block(value, mask),
            ExecNode::List(items) => {
                let blocks = items
                    .iter()
                    .map(|item| self.eval_node(*item, mask))
                    .collect();
                eval_list(blocks, mask)
            }
            ExecNode::Input(slot) => {
                let Some(column) = self.inputs.column(slot).cloned() else {
                    return EvalBlock::fail_mask(mask, EvalError::PropertyDisabled);
                };
                EvalBlock::new(
                    column.normalize_inactive(mask),
                    Mask::all(mask.len()),
                    Vec::new(),
                )
            }
            ExecNode::Variable(name) => {
                let column = self
                    .scopes
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(&name))
                    .cloned();
                match column {
                    Some(column) => EvalBlock::new(
                        column.normalize_inactive(mask),
                        Mask::all(mask.len()),
                        Vec::new(),
                    ),
                    None => EvalBlock::fail_mask(mask, EvalError::InvalidArgument),
                }
            }
            ExecNode::Unary { op, input } => {
                let input = self.eval_node(input, mask);
                eval_unary(op, input, mask)
            }
            ExecNode::Binary {
                op: analyzer::ast::BinOpKind::AndAnd,
                left,
                right,
            } => {
                let left = self.eval_node(left, mask);
                eval_logical_and(left, mask, |right_mask| self.eval_node(right, right_mask))
            }
            ExecNode::Binary {
                op: analyzer::ast::BinOpKind::OrOr,
                left,
                right,
            } => {
                let left = self.eval_node(left, mask);
                eval_logical_or(left, mask, |right_mask| self.eval_node(right, right_mask))
            }
            ExecNode::Binary { op, left, right } => {
                let left = self.eval_node(left, mask);
                let right = self.eval_node(right, mask);
                eval_binary(op, left, right, mask)
            }
            ExecNode::Ternary {
                condition,
                then_plan,
                else_plan,
            } => {
                let condition = self.eval_node(condition, mask);
                let (then_mask, else_mask) = super::operators::split_condition(&condition, mask);
                let then_block = self.eval_node(then_plan, &then_mask);
                let else_block = self.eval_node(else_plan, &else_mask);
                super::operators::merge_condition(
                    condition, then_block, else_block, mask, &then_mask,
                )
            }
            ExecNode::Cast { input, target } => {
                let input = self.eval_node(input, mask);
                eval_cast(input, target, mask)
            }
            ExecNode::Builtin(call) => match call.key.evaluation_mode() {
                BuiltinEvaluationMode::Value => {
                    let mut arguments = Vec::with_capacity(call.arguments.len());
                    for argument in &call.arguments {
                        let PlannedArgumentKind::Value(plan) = argument.kind else {
                            return EvalBlock::fail_mask(mask, EvalError::TypeMismatch);
                        };
                        arguments.push(EvaluatedArgument {
                            parameter: argument.parameter,
                            repeat_group: argument.repeat_group,
                            block: self.eval_node(plan, mask),
                        });
                    }
                    let prepared = PreparedValueArguments::new(
                        arguments,
                        mask,
                        call.key,
                        call.debug_contract(),
                    );
                    let context = BuiltinKernelContext::new(self.inputs.runtime(), self.batch);
                    dispatch_value(call.key, prepared, &context, call.debug_contract())
                }
                BuiltinEvaluationMode::Controlled => {
                    let prepared = PreparedControlledArguments::new(
                        self.plan.owner(),
                        &call.arguments,
                        call.debug_contract(),
                    );
                    dispatch_controlled(call.key, self, prepared, mask, call.debug_contract())
                }
            },
        }
    }

    fn validate_handle(&self, owner: crate::ir::PlanOwner, mask: &Mask) -> bool {
        owner == self.plan.owner() && mask.len() == self.batch.len()
    }
}

impl BuiltinEvalContext for Runtime<'_> {
    fn plan_owner(&self) -> crate::ir::PlanOwner {
        self.plan.owner()
    }

    fn eval<K: ColumnKind>(&mut self, plan: ValuePlan<K>, mask: &Mask) -> KernelResult<K> {
        let (owner, id, contract) = plan.into_parts();
        if !self.validate_handle(owner, mask) {
            return block_into_kernel(EvalBlock::fail_mask(mask, EvalError::InvalidArgument), mask);
        }
        let block = self.eval_node(id, mask);
        assert_materialized_argument(contract.as_ref(), &block, mask);
        block_into_kernel(block, mask)
    }

    fn eval_thunk<K: ColumnKind>(&mut self, plan: ThunkPlan<K>, mask: &Mask) -> KernelResult<K> {
        let (owner, body, contract) = plan.into_parts();
        if !self.validate_handle(owner, mask) {
            return block_into_kernel(EvalBlock::fail_mask(mask, EvalError::InvalidArgument), mask);
        }
        let block = self.eval_node(body, mask);
        assert_materialized_argument(contract.as_ref(), &block, mask);
        block_into_kernel(block, mask)
    }

    fn apply_lambda<K: ColumnKind>(
        &mut self,
        plan: LambdaPlan<K>,
        bindings: LambdaBindings,
        mask: &Mask,
    ) -> KernelResult<K> {
        let (owner, body, parameters, contract) = plan.into_parts();
        if !self.validate_handle(owner, mask)
            || parameters.len() != bindings.as_slice().len()
            || bindings
                .as_slice()
                .iter()
                .any(|(_, column)| column.len() != mask.len())
        {
            return block_into_kernel(EvalBlock::fail_mask(mask, EvalError::InvalidArgument), mask);
        }
        let scope = parameters
            .iter()
            .zip(bindings.as_slice())
            .map(|(expected_name, (provided_name, column))| {
                debug_assert_eq!(expected_name, provided_name);
                (expected_name.clone(), column.clone())
            })
            .collect();
        self.scopes.push(scope);
        let block = self.eval_node(body, mask);
        self.scopes.pop();
        assert_materialized_argument(contract.as_ref(), &block, mask);
        block_into_kernel(block, mask)
    }
}
