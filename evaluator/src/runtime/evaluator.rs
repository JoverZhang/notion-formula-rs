use analyzer::ast::Expr;

use crate::core::context::EvalContext;
use crate::core::errors::{EvalError, ProviderError, SimpleEvalError};
use crate::core::provider::Provider;
use crate::core::types::{ColumnBlock, EvalBlock, Mask, RowBatch};
use crate::ir::nodes::ExecNode;
use crate::planner::Planner;

use super::cast::cast_block_to_f64;
use super::dispatch::dispatch_binary;
use super::literals::{literal_any, literal_f64};

#[derive(Debug)]
pub struct Evaluator<'a, P: Provider> {
    ctx: &'a EvalContext,
    _provider: &'a P,
    planner: Planner,
}

impl<'a, P: Provider> Evaluator<'a, P> {
    pub fn new(ctx: &'a EvalContext, provider: &'a P) -> Self {
        Self {
            ctx,
            _provider: provider,
            planner: Planner,
        }
    }

    pub async fn eval(&self, expr: &Expr, batch: RowBatch<'_>) -> Result<EvalBlock, ProviderError> {
        self.eval_with_mask(expr, batch, vec![true; batch.rows.len()])
            .await
    }

    pub async fn eval_with_mask(
        &self,
        expr: &Expr,
        batch: RowBatch<'_>,
        mask: Mask,
    ) -> Result<EvalBlock, ProviderError> {
        if mask.len() != batch.rows.len() {
            return Err(ProviderError::BackendError);
        }

        let plan = match self.planner.build(expr, self.ctx) {
            Ok(plan) => plan,
            Err(error) => return Ok(EvalBlock::fail_mask(&mask, error.into())),
        };

        Ok(self.eval_node(&plan.root, batch.rows.len(), &mask))
    }

    pub async fn eval_simple_fail_batch(
        &self,
        expr: &Expr,
        batch: RowBatch<'_>,
    ) -> Result<ColumnBlock, SimpleEvalError> {
        let out = self
            .eval(expr, batch)
            .await
            .map_err(SimpleEvalError::Provider)?;

        if let Some((row_index, reason)) = out.errors.first().cloned() {
            return Err(SimpleEvalError::FirstRowError {
                row_index,
                reason,
                total: out.len(),
            });
        }

        if let Some(row_index) = out.ok.iter().position(|row_ok| !row_ok) {
            return Err(SimpleEvalError::FirstRowError {
                row_index,
                reason: EvalError::InvalidArgument,
                total: out.len(),
            });
        }

        Ok(out.values)
    }

    fn eval_node(&self, node: &ExecNode, len: usize, mask: &Mask) -> EvalBlock {
        match node {
            ExecNode::LiteralF64(value) => literal_f64(*value, len, mask),
            ExecNode::LiteralAny(value) => literal_any(value.clone(), len, mask),
            ExecNode::CastToF64 { input } => {
                let input = self.eval_node(input, len, mask);
                cast_block_to_f64(input, mask)
            }
            ExecNode::Binary { key, left, right } => {
                let left = self.eval_node(left, len, mask);
                let right = self.eval_node(right, len, mask);
                dispatch_binary(*key, left, right, mask)
            }
        }
    }
}
