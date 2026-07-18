use std::collections::HashMap;

use analyzer::SemanticMap;
use analyzer::analysis::{ShapeValidity, Ty, param_for_ref};
use analyzer::ast::{Expr, ExprKind};
use analyzer::{LitKind, NodeId};

use crate::builtins::BuiltinKey;
use crate::core::columns::AbiKind;
use crate::core::context::EvalContext;
use crate::core::errors::PrepareError;
use crate::core::inputs::{InputLayoutId, InputSlot, RequiredColumn, abi_kind_for_ty};
use crate::core::types::Value;
use crate::ir::{
    BuiltinCallNode, DebugArgumentContract, DebugCallContract, ExecNode, ExecPlan, PlanId,
    PlanOwner, PlannedArgument, PlannedArgumentKind,
};

pub(crate) struct Planner<'a> {
    context: &'a EvalContext,
    semantic: &'a SemanticMap,
    functions: HashMap<String, builtin_fn::FunctionSig>,
    layout: InputLayoutId,
    nodes: Vec<ExecNode>,
    required_columns: Vec<RequiredColumn>,
    property_slots: HashMap<String, InputSlot>,
}

impl<'a> Planner<'a> {
    pub(crate) fn new(
        context: &'a EvalContext,
        semantic: &'a SemanticMap,
        layout: InputLayoutId,
    ) -> Self {
        Self {
            context,
            semantic,
            functions: builtin_fn::builtins_functions()
                .into_iter()
                .map(|signature| (signature.name.clone(), signature))
                .collect(),
            layout,
            nodes: Vec::new(),
            required_columns: Vec::new(),
            property_slots: HashMap::new(),
        }
    }

    pub(crate) fn build(
        mut self,
        expression: &Expr,
    ) -> Result<(ExecPlan, Box<[RequiredColumn]>), PrepareError> {
        let root = self.lower(expression)?;
        let output_abi = self
            .semantic
            .expression_types
            .get(expression.id)
            .map(abi_kind_for_ty)
            .unwrap_or(AbiKind::Any);
        let root = self.ensure_abi(root, output_abi);
        let owner = PlanOwner::next();
        Ok((
            ExecPlan::new(owner, self.nodes, root),
            self.required_columns.into_boxed_slice(),
        ))
    }

    fn lower(&mut self, expression: &Expr) -> Result<PlanId, PrepareError> {
        match &expression.kind {
            ExprKind::Group { inner } => self.lower(inner),
            ExprKind::Lit(literal) => {
                let value = match literal.kind {
                    LitKind::Number => literal
                        .symbol
                        .text
                        .parse::<f64>()
                        .map(Value::Number)
                        .map_err(|_| PrepareError::UnsupportedExpression)?,
                    LitKind::String => Value::Text(literal.symbol.text.clone()),
                    LitKind::Bool => Value::Bool(literal.symbol.text == "true"),
                };
                Ok(self.push(ExecNode::Literal(value)))
            }
            ExprKind::List { items } => {
                let items = items
                    .iter()
                    .map(|item| self.lower(item))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.push(ExecNode::List(items.into_boxed_slice())))
            }
            ExprKind::Ident(symbol) => Ok(self.push(ExecNode::Variable(symbol.text.clone()))),
            ExprKind::Unary { op, expr } => {
                let input = self.lower(expr)?;
                Ok(self.push(ExecNode::Unary { op: *op, input }))
            }
            ExprKind::Binary { op, left, right } => {
                let left = self.lower(left)?;
                let right = self.lower(right)?;
                Ok(self.push(ExecNode::Binary {
                    op: op.node,
                    left,
                    right,
                }))
            }
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
            } => {
                let condition = self.lower(cond)?;
                let then_plan = self.lower(then)?;
                let else_plan = self.lower(otherwise)?;
                Ok(self.push(ExecNode::Ternary {
                    condition,
                    then_plan,
                    else_plan,
                }))
            }
            ExprKind::Call { callee, args } if callee.text == "prop" => self.lower_property(args),
            ExprKind::Call { callee, args } => {
                self.lower_builtin(expression.id, &callee.text, args)
            }
            ExprKind::ImplicitLambda { body, .. } => self.lower(body),
            ExprKind::MemberCall { .. } | ExprKind::Error => {
                Err(PrepareError::UnsupportedExpression)
            }
        }
    }

    fn lower_property(&mut self, args: &[Expr]) -> Result<PlanId, PrepareError> {
        let [argument] = args else {
            return Err(PrepareError::UnsupportedExpression);
        };
        let ExprKind::Lit(literal) = &argument.kind else {
            return Err(PrepareError::UnsupportedExpression);
        };
        if literal.kind != LitKind::String {
            return Err(PrepareError::UnsupportedExpression);
        }
        let name = literal.symbol.text.clone();
        let property = self
            .context
            .property(&name)
            .ok_or_else(|| PrepareError::UnknownProperty(name.clone()))?;
        let slot = if let Some(slot) = self.property_slots.get(&name) {
            *slot
        } else {
            let slot = InputSlot::new(self.layout, self.required_columns.len());
            self.required_columns.push(RequiredColumn {
                slot,
                name: name.clone(),
                expected_type: property.ty.clone(),
            });
            self.property_slots.insert(name, slot);
            slot
        };
        Ok(self.push(ExecNode::Input(slot)))
    }

    fn lower_builtin(
        &mut self,
        call_id: NodeId,
        name: &str,
        args: &[Expr],
    ) -> Result<PlanId, PrepareError> {
        let signature = self
            .functions
            .get(name)
            .cloned()
            .ok_or(PrepareError::UnsupportedExpression)?;
        let resolved = self
            .semantic
            .builtin_calls
            .get(&call_id)
            .ok_or(PrepareError::MissingResolvedCall)?;
        if !matches!(resolved.validity, ShapeValidity::Valid) {
            return Err(PrepareError::InvalidResolvedShape);
        }
        if resolved.arguments.len() != args.len() {
            return Err(PrepareError::InvalidResolvedShape);
        }

        let key = BuiltinKey::from_name(name).ok_or(PrepareError::UnsupportedExpression)?;
        debug_assert_eq!(key.return_abi(), abi_kind_for_ty(&signature.ret));
        let mut arguments = Vec::with_capacity(args.len());
        for (argument, resolved_argument) in args.iter().zip(&resolved.arguments) {
            let parameter = resolved_argument
                .parameter
                .ok_or(PrepareError::InvalidResolvedShape)?;
            let template = param_for_ref(&signature, parameter);
            let kind = match &template.ty {
                Ty::Fn { params, ret } => {
                    let ExprKind::ImplicitLambda {
                        params: inferred_params,
                        body,
                    } = &argument.kind
                    else {
                        return Err(PrepareError::UnsupportedExpression);
                    };
                    let body = self.lower(body)?;
                    let body = self.ensure_abi(body, abi_kind_for_ty(ret));
                    if params.is_empty() {
                        PlannedArgumentKind::Thunk { body }
                    } else {
                        PlannedArgumentKind::Lambda {
                            body,
                            parameters: inferred_params.clone().into_boxed_slice(),
                        }
                    }
                }
                Ty::Ident(_) => {
                    let ExprKind::Ident(symbol) = &argument.kind else {
                        return Err(PrepareError::UnsupportedExpression);
                    };
                    PlannedArgumentKind::Binder {
                        name: symbol.text.clone(),
                    }
                }
                _ => {
                    let value = self.lower(argument)?;
                    PlannedArgumentKind::Value(
                        self.ensure_abi(value, abi_kind_for_ty(&template.ty)),
                    )
                }
            };
            arguments.push(PlannedArgument {
                parameter,
                repeat_group: resolved_argument.repeat_group,
                kind,
            });
        }

        #[cfg(debug_assertions)]
        let debug_contract = DebugCallContract {
            arguments: resolved
                .arguments
                .iter()
                .filter_map(|argument| {
                    Some(DebugArgumentContract {
                        parameter: argument.parameter?,
                        repeat_group: argument.repeat_group,
                        expected_ty: argument.expected_ty.clone()?,
                    })
                })
                .collect(),
            return_ty: resolved.return_ty.clone(),
        };

        Ok(self.push(ExecNode::Builtin(BuiltinCallNode {
            key,
            arguments: arguments.into_boxed_slice(),
            #[cfg(debug_assertions)]
            debug_contract,
        })))
    }

    fn push(&mut self, node: ExecNode) -> PlanId {
        let id = PlanId::new(self.nodes.len());
        self.nodes.push(node);
        id
    }

    fn ensure_abi(&mut self, input: PlanId, target: AbiKind) -> PlanId {
        if self.node_abi(input) == target {
            input
        } else {
            self.push(ExecNode::Cast { input, target })
        }
    }

    fn node_abi(&self, id: PlanId) -> AbiKind {
        match &self.nodes[id.index()] {
            ExecNode::Literal(Value::Number(_)) => AbiKind::Number,
            ExecNode::Literal(Value::Text(_)) => AbiKind::Text,
            ExecNode::Literal(Value::Bool(_)) => AbiKind::Boolean,
            ExecNode::Literal(Value::Date(_)) => AbiKind::Date,
            ExecNode::Literal(Value::List(_)) | ExecNode::List(_) => AbiKind::List,
            ExecNode::Input(slot) => self
                .required_columns
                .get(slot.index())
                .map(|column| abi_kind_for_ty(&column.expected_type))
                .unwrap_or(AbiKind::Any),
            ExecNode::Variable(_)
            | ExecNode::Unary { .. }
            | ExecNode::Binary { .. }
            | ExecNode::Ternary { .. } => AbiKind::Any,
            ExecNode::Cast { target, .. } => *target,
            ExecNode::Builtin(call) => call.key.return_abi(),
        }
    }
}
