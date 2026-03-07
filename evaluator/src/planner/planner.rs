use analyzer::analysis::{Context as SemaContext, Ty, TypeMap, infer_expr_with_map};
use analyzer::ast::{BinOpKind, Expr, ExprKind};

use crate::core::context::EvalContext;
use crate::ir::nodes::{CastPlan, ExecNode, ExecPlan};

use super::PlanError;
use super::lower_lit::{lower_const_value, lower_lit};
use super::selectors::select_binary_plan;

#[derive(Debug, Default)]
pub(crate) struct Planner;

impl Planner {
    pub(crate) fn build(&self, expr: &Expr, ctx: &EvalContext) -> Result<ExecPlan, PlanError> {
        let sema_ctx = SemaContext {
            properties: ctx.properties.clone(),
            functions: vec![],
        };
        let mut map = TypeMap::default();
        let _ = infer_expr_with_map(expr, &sema_ctx, &mut map);
        let root = self.lower(expr, &map)?;
        Ok(ExecPlan { root })
    }

    fn lower(&self, expr: &Expr, map: &TypeMap) -> Result<ExecNode, PlanError> {
        match &expr.kind {
            ExprKind::Group { inner } => self.lower(inner, map),
            ExprKind::Lit(lit) => lower_lit(lit),
            ExprKind::List { .. } => lower_const_value(expr).map(ExecNode::LiteralAny),
            ExprKind::Binary { op, left, right } if is_arithmetic_op(op.node) => {
                let left_node = self.lower(left, map)?;
                let right_node = self.lower(right, map)?;
                let left_ty = inferred_ty_for_expr(map, left)?;
                let right_ty = inferred_ty_for_expr(map, right)?;
                let plan = select_binary_plan(op.node, &left_ty, &right_ty)?;

                Ok(ExecNode::Binary {
                    key: plan.key,
                    left: Box::new(apply_cast(left_node, plan.left_cast)),
                    right: Box::new(apply_cast(right_node, plan.right_cast)),
                })
            }
            _ => Err(PlanError::InvalidArgument),
        }
    }
}

fn inferred_ty_for_expr(map: &TypeMap, expr: &Expr) -> Result<Ty, PlanError> {
    map.get(expr.id)
        .cloned()
        .ok_or(PlanError::MissingTypeMapEntry)
}

fn apply_cast(node: ExecNode, cast: CastPlan) -> ExecNode {
    match cast {
        CastPlan::None => node,
        CastPlan::ToF64 => ExecNode::CastToF64 {
            input: Box::new(node),
        },
    }
}

fn is_arithmetic_op(op: BinOpKind) -> bool {
    matches!(
        op,
        BinOpKind::Plus | BinOpKind::Minus | BinOpKind::Star | BinOpKind::Slash
    )
}
