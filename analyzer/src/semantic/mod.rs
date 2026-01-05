use crate::ast::{Expr, ExprKind};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::token::{LitKind, Span};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Ty {
    Number,
    String,
    Boolean,
    Date,
    Null,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Ty,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub properties: Vec<Property>,
}

impl Context {
    pub fn lookup(&self, name: &str) -> Option<Ty> {
        self.properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.ty)
    }
}

pub fn analyze_expr(expr: &Expr, ctx: &Context) -> (Ty, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let ty = analyze_expr_inner(expr, ctx, &mut diags);
    (ty, diags)
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
                Dot => Ty::Unknown,
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
            "if" => analyze_if(expr, args, ctx, diags),
            "sum" => analyze_sum(expr, args, ctx, diags),
            _ => {
                for arg in args {
                    let _ = analyze_expr_inner(arg, ctx, diags);
                }
                Ty::Unknown
            }
        },
        ExprKind::Error => Ty::Unknown,
    }
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

    if cond_ty != Ty::Boolean {
        emit_error(diags, args[0].span, "if() condition must be boolean");
    }

    join_types(then_ty, otherwise_ty)
}

fn analyze_sum(expr: &Expr, args: &[Expr], ctx: &Context, diags: &mut Vec<Diagnostic>) -> Ty {
    if args.is_empty() {
        emit_error(diags, expr.span, "sum() expects at least 1 argument");
        return Ty::Number;
    }

    let mut arg_tys = Vec::with_capacity(args.len());
    for arg in args {
        arg_tys.push(analyze_expr_inner(arg, ctx, diags));
    }

    for (arg, ty) in args.iter().zip(arg_tys.iter()) {
        if *ty != Ty::Number {
            emit_error(diags, arg.span, "sum() expects number arguments");
        }
    }

    Ty::Number
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
