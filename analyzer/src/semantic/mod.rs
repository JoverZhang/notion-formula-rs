use crate::ast::{Expr, ExprKind};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::token::{LitKind, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;

mod functions;
pub use functions::builtins_functions;

static POSTFIX_CAPABLE_BUILTIN_NAMES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    builtins_functions()
        .into_iter()
        // A builtin is postfix-capable if it has at least one non-receiver parameter.
        // Postfix form: `receiver.fn(arg1, ...)` corresponds to `fn(receiver, arg1, ...)`.
        .filter(|sig| sig.params.len() > 1)
        .map(|sig| sig.name)
        .collect()
});

pub fn postfix_capable_builtin_names() -> &'static HashSet<String> {
    &POSTFIX_CAPABLE_BUILTIN_NAMES
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Ty {
    Number,
    String,
    Boolean,
    Date,
    Null,
    Unknown,
    List(Box<Ty>),
    Union(Vec<Ty>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FunctionCategory {
    General,
    Text,
    Number,
    Date,
    People,
    List,
    Special,
}

pub fn ty_accepts(expected: &Ty, actual: &Ty) -> bool {
    if matches!(expected, Ty::Unknown) || matches!(actual, Ty::Unknown) {
        return true;
    }
    match (expected, actual) {
        (Ty::Union(branches), actual) => branches.iter().any(|t| ty_accepts(t, actual)),
        (Ty::List(e), Ty::List(a)) => ty_accepts(e, a),
        _ => expected == actual,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<ParamSig>,
    pub ret: Ty,
    pub detail: Option<String>,
    pub min_args: usize,
    pub category: FunctionCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamSig {
    pub name: Option<String>,
    pub ty: Ty,
    pub optional: bool,
    pub variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Ty,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub properties: Vec<Property>,
    pub functions: Vec<FunctionSig>,
}

impl FunctionSig {
    pub fn is_variadic(&self) -> bool {
        self.params.last().is_some_and(|p| p.variadic)
    }

    pub fn fixed_params_len(&self) -> usize {
        if self.is_variadic() {
            self.params.len().saturating_sub(1)
        } else {
            self.params.len()
        }
    }

    pub fn effective_min_args(&self) -> usize {
        if self.min_args > 0 {
            return self.min_args;
        }
        if self.is_variadic() {
            self.fixed_params_len()
        } else {
            self.params.len()
        }
    }

    pub fn param_for_arg_index(&self, idx: usize) -> Option<&ParamSig> {
        if idx < self.params.len() {
            return self.params.get(idx);
        }
        if self.is_variadic() {
            return self.params.last();
        }
        None
    }
}

impl Context {
    pub fn lookup(&self, name: &str) -> Option<Ty> {
        self.properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.ty.clone())
    }
}

pub fn analyze_expr(expr: &Expr, ctx: &Context) -> (Ty, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let ty = analyze_expr_inner(expr, ctx, &mut diags);
    (ty, diags)
}

fn lookup_function<'a>(ctx: &'a Context, name: &str) -> Option<&'a FunctionSig> {
    ctx.functions.iter().find(|f| f.name == name)
}

fn analyze_expr_inner(expr: &Expr, ctx: &Context, diags: &mut Vec<Diagnostic>) -> Ty {
    match &expr.kind {
        ExprKind::Lit(lit) => match lit.kind {
            LitKind::Number => Ty::Number,
            LitKind::String => Ty::String,
            LitKind::Bool => Ty::Boolean,
        },
        ExprKind::Ident(_) => Ty::Unknown,
        ExprKind::Group { inner } => analyze_expr_inner(inner, ctx, diags),
        ExprKind::MemberCall {
            receiver,
            method,
            args,
        } => {
            // Phase 8: minimal typing. For `.if(cond, otherwise)` we treat it like:
            // `condition.if(then, else)` is treated like `if(condition, then, else)` (receiver is the `condition`).
            if method.text == "if" && args.len() == 2 {
                if lookup_function(ctx, "if").is_none() {
                    let _ = analyze_expr_inner(receiver, ctx, diags);
                    for arg in args {
                        let _ = analyze_expr_inner(arg, ctx, diags);
                    }
                    emit_error(diags, expr.span, "unknown function: if");
                    return Ty::Unknown;
                }

                let cond_ty = analyze_expr_inner(receiver, ctx, diags);
                let then_ty = analyze_expr_inner(&args[0], ctx, diags);
                let otherwise_ty = analyze_expr_inner(&args[1], ctx, diags);
                if cond_ty != Ty::Unknown && cond_ty != Ty::Boolean {
                    emit_error(diags, receiver.span, "if() condition must be boolean");
                }
                join_types(then_ty, otherwise_ty)
            } else {
                let _ = analyze_expr_inner(receiver, ctx, diags);
                for arg in args {
                    let _ = analyze_expr_inner(arg, ctx, diags);
                }
                Ty::Unknown
            }
        }
        ExprKind::Unary { op, expr } => {
            let inner_ty = analyze_expr_inner(expr, ctx, diags);
            match op.node {
                crate::ast::UnOpKind::Not => match inner_ty {
                    Ty::Boolean => Ty::Boolean,
                    _ => Ty::Unknown,
                },
                crate::ast::UnOpKind::Neg => match inner_ty {
                    Ty::Number => Ty::Number,
                    _ => Ty::Unknown,
                },
            }
        }
        ExprKind::Binary { op, left, right } => {
            let left_ty = analyze_expr_inner(left, ctx, diags);
            let right_ty = analyze_expr_inner(right, ctx, diags);
            use crate::ast::BinOpKind::*;
            match op.node {
                Plus | Minus | Star | Slash | Percent | Caret => {
                    if left_ty == Ty::Number && right_ty == Ty::Number {
                        Ty::Number
                    } else {
                        Ty::Unknown
                    }
                }
                AndAnd | OrOr => {
                    if left_ty == Ty::Boolean && right_ty == Ty::Boolean {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
                Lt | Le | Ge | Gt => {
                    if left_ty != Ty::Unknown && right_ty != Ty::Unknown {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
                EqEq | Ne => {
                    if left_ty == right_ty && left_ty != Ty::Unknown {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
            }
        }
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            let _ = analyze_expr_inner(cond, ctx, diags);
            let then_ty = analyze_expr_inner(then, ctx, diags);
            let otherwise_ty = analyze_expr_inner(otherwise, ctx, diags);
            join_types(then_ty, otherwise_ty)
        }
        ExprKind::Call { callee, args } => match callee.text.as_str() {
            "prop" => analyze_prop(expr, args, ctx, diags),
            name => {
                let Some(sig) = lookup_function(ctx, name) else {
                    for arg in args {
                        let _ = analyze_expr_inner(arg, ctx, diags);
                    }
                    emit_error(diags, expr.span, format!("unknown function: {}", name));
                    return Ty::Unknown;
                };

                match name {
                    "if" => analyze_if(expr, args, ctx, diags),
                    _ => analyze_call(expr, sig, args, ctx, diags),
                }
            }
        },
        ExprKind::Error => Ty::Unknown,
    }
}

fn analyze_call(
    expr: &Expr,
    sig: &FunctionSig,
    args: &[Expr],
    ctx: &Context,
    diags: &mut Vec<Diagnostic>,
) -> Ty {
    debug_assert!(
        sig.params
            .iter()
            .take(sig.params.len().saturating_sub(1))
            .all(|p| !p.variadic),
        "only the last param may be variadic"
    );

    let mut arg_tys = Vec::with_capacity(args.len());
    for arg in args {
        arg_tys.push(analyze_expr_inner(arg, ctx, diags));
    }

    if sig.is_variadic() {
        let required = sig.effective_min_args().max(sig.fixed_params_len());
        if args.len() < required {
            let plural = if required == 1 { "" } else { "s" };
            emit_error(
                diags,
                expr.span,
                format!(
                    "{}() expects at least {} argument{}",
                    sig.name, required, plural
                ),
            );
        }
    } else if args.len() != sig.params.len() {
        let expected = sig.params.len();
        let plural = if expected == 1 { "" } else { "s" };
        emit_error(
            diags,
            expr.span,
            format!(
                "{}() expects exactly {} argument{}",
                sig.name, expected, plural
            ),
        );
    }

    for (idx, (arg, ty)) in args.iter().zip(arg_tys.iter()).enumerate() {
        let Some(param) = sig.param_for_arg_index(idx) else {
            continue;
        };
        if !ty_accepts(&param.ty, ty) {
            if sig.name == "sum" {
                emit_error(diags, arg.span, "sum() expects number arguments");
            } else {
                emit_error(
                    diags,
                    arg.span,
                    format!(
                        "argument type mismatch: expected {:?}, got {:?}",
                        param.ty, ty
                    ),
                );
            }
        }
    }

    sig.ret.clone()
}

fn analyze_prop(expr: &Expr, args: &[Expr], ctx: &Context, diags: &mut Vec<Diagnostic>) -> Ty {
    for arg in args {
        let _ = analyze_expr_inner(arg, ctx, diags);
    }

    if args.len() != 1 {
        emit_error(diags, expr.span, "prop() expects exactly 1 argument");
        return Ty::Unknown;
    }

    let arg = &args[0];
    let name = match &arg.kind {
        ExprKind::Lit(lit) if lit.kind == LitKind::String => lit.symbol.text.as_str(),
        _ => {
            emit_error(diags, arg.span, "prop() expects a string literal argument");
            return Ty::Unknown;
        }
    };

    match ctx.lookup(name) {
        Some(ty) => ty,
        None => {
            emit_error(diags, arg.span, format!("Unknown property: {}", name));
            Ty::Unknown
        }
    }
}

fn analyze_if(expr: &Expr, args: &[Expr], ctx: &Context, diags: &mut Vec<Diagnostic>) -> Ty {
    if args.len() != 3 {
        for arg in args {
            let _ = analyze_expr_inner(arg, ctx, diags);
        }
        emit_error(diags, expr.span, "if() expects exactly 3 arguments");
        return Ty::Unknown;
    }

    let cond_ty = analyze_expr_inner(&args[0], ctx, diags);
    let then_ty = analyze_expr_inner(&args[1], ctx, diags);
    let otherwise_ty = analyze_expr_inner(&args[2], ctx, diags);

    if cond_ty != Ty::Unknown && cond_ty != Ty::Boolean {
        emit_error(diags, args[0].span, "if() condition must be boolean");
    }

    join_types(then_ty, otherwise_ty)
}

fn join_types(a: Ty, b: Ty) -> Ty {
    if a == Ty::Unknown || b == Ty::Unknown {
        Ty::Unknown
    } else if a == b {
        a
    } else {
        Ty::Unknown
    }
}

fn emit_error(diags: &mut Vec<Diagnostic>, span: Span, message: impl Into<String>) {
    diags.push(Diagnostic {
        kind: DiagnosticKind::Error,
        message: message.into(),
        span,
        labels: vec![],
        notes: vec![],
    });
}
