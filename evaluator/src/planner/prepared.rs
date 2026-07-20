use analyzer::analysis::{Context as SemanticContext, analyze_expr_with_semantic_map};
use analyzer::ast::Expr;

use crate::core::context::EvalContext;
use crate::core::errors::{InputContractError, PrepareError};
use crate::core::inputs::{EvalInputs, InputLayoutId, RequiredColumn};
use crate::core::types::{EvalBlock, Mask, RowBatch};
use crate::ir::ExecPlan;
use crate::runtime::Runtime;

use super::Planner;

pub struct PreparedFormula {
    pub(crate) plan: ExecPlan,
    required_columns: Box<[RequiredColumn]>,
    input_layout: InputLayoutId,
}

impl std::fmt::Debug for PreparedFormula {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedFormula")
            .field("required_columns", &self.required_columns)
            .finish_non_exhaustive()
    }
}

pub fn prepare_formula(
    expression: &mut Expr,
    context: &EvalContext,
) -> Result<PreparedFormula, PrepareError> {
    let semantic_context = SemanticContext {
        properties: context.properties.clone(),
        functions: builtin_fn::builtins_functions(),
    };
    let (_, semantic_map, diagnostics) =
        analyze_expr_with_semantic_map(expression, &semantic_context);
    if !diagnostics.is_empty() {
        return Err(PrepareError::Semantic(
            diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.message)
                .collect(),
        ));
    }

    let input_layout = InputLayoutId::next();
    let (plan, required_columns) =
        Planner::new(context, &semantic_map, input_layout).build(expression)?;
    Ok(PreparedFormula {
        plan,
        required_columns,
        input_layout,
    })
}

impl PreparedFormula {
    pub fn required_columns(&self) -> &[RequiredColumn] {
        &self.required_columns
    }

    pub(crate) fn input_layout_id(&self) -> InputLayoutId {
        self.input_layout
    }

    pub fn evaluate(
        &self,
        batch: RowBatch,
        inputs: EvalInputs,
    ) -> Result<EvalBlock, InputContractError> {
        let mask = Mask::all(batch.len());
        self.evaluate_with_mask(batch, inputs, mask)
    }

    pub fn evaluate_with_mask(
        &self,
        batch: RowBatch,
        inputs: EvalInputs,
        mask: Mask,
    ) -> Result<EvalBlock, InputContractError> {
        if inputs.layout() != self.input_layout
            || inputs.batch_len() != batch.len()
            || mask.len() != batch.len()
        {
            return Err(InputContractError::WrongInputLayout);
        }
        Ok(Runtime::new(&self.plan, &batch, &inputs).evaluate(&mask))
    }
}
