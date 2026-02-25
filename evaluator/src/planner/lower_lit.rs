use analyzer::{LitKind, ast::ExprKind};

use crate::core::types::Value;
use crate::ir::nodes::ExecNode;

use super::PlanError;

pub(crate) fn lower_lit(lit: &analyzer::Lit) -> Result<ExecNode, PlanError> {
    match lit.kind {
        LitKind::Number => lit
            .symbol
            .text
            .parse::<f64>()
            .map(ExecNode::LiteralF64)
            .map_err(|_| PlanError::InvalidArgument),
        LitKind::String => Ok(ExecNode::LiteralAny(Value::Text(lit.symbol.text.clone()))),
        LitKind::Bool => match lit.symbol.text.as_str() {
            "true" => Ok(ExecNode::LiteralAny(Value::Bool(true))),
            "false" => Ok(ExecNode::LiteralAny(Value::Bool(false))),
            _ => Err(PlanError::InvalidArgument),
        },
    }
}

pub(crate) fn lower_const_value(expr: &analyzer::ast::Expr) -> Result<Value, PlanError> {
    match &expr.kind {
        ExprKind::Group { inner } => lower_const_value(inner),
        ExprKind::Lit(lit) => match lit.kind {
            LitKind::Number => lit
                .symbol
                .text
                .parse::<f64>()
                .map(Value::Number)
                .map_err(|_| PlanError::InvalidArgument),
            LitKind::String => Ok(Value::Text(lit.symbol.text.clone())),
            LitKind::Bool => match lit.symbol.text.as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                _ => Err(PlanError::InvalidArgument),
            },
        },
        ExprKind::List { items } => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(lower_const_value(item)?);
            }
            Ok(Value::List(out))
        }
        _ => Err(PlanError::InvalidArgument),
    }
}
