//! Pre-inference AST desugaring passes.
//!
//! Rewrites supported postfix calls and sequential `lets` bindings before inference.
//!
//! This runs *before* type inference so that `infer_call` can mutate argument nodes
//! in-place (e.g. wrapping them in [`ExprKind::ImplicitLambda`]) without the mutations
//! being lost on a clone.

use crate::ast::{Expr, ExprKind};
use crate::lexer::{Span, Symbol};

/// Rewrite supported postfix calls and sequential bindings before inference.
///
/// This mutates the AST in-place. After this pass, every `MemberCall` whose method is a
/// postfix-capable builtin has been replaced by an equivalent `Call` node with the
/// receiver prepended to the argument list.
/// Valid `lets` calls become nested `let` calls, with each original argument retained.
///
/// Non-builtin member calls (and builtins that are not postfix-capable) are left
/// untouched.
pub fn desugar_member_calls(expr: &mut Expr) {
    // Recurse into children first (post-order) so nested member calls are desugared
    // before we inspect the current node.
    desugar_children(expr);

    // Now check if this node itself is a desugable MemberCall.
    let should_desugar = matches!(
        &expr.kind,
        ExprKind::MemberCall { method, .. }
            if super::postfix_capable_builtin_names().contains(method.text.as_str())
    );

    if should_desugar {
        // Take ownership of the current kind, replacing it temporarily with Error.
        let old_kind = std::mem::replace(&mut expr.kind, ExprKind::Error);
        let ExprKind::MemberCall {
            receiver,
            method,
            mut args,
        } = old_kind
        else {
            unreachable!();
        };

        // Build prefix-form args: [receiver, ...original_args]
        let mut new_args = Vec::with_capacity(1 + args.len());
        new_args.push(*receiver);
        new_args.append(&mut args);

        expr.kind = ExprKind::Call {
            callee: method,
            args: new_args,
        };
    }

    desugar_sequential_bindings(expr);
}

fn desugar_sequential_bindings(expr: &mut Expr) {
    let valid = matches!(&expr.kind, ExprKind::Call { callee, args }
        if callee.text == "lets" && args.len() >= 3 && args.len() % 2 == 1
        && args[..args.len() - 1].iter().step_by(2).all(|arg| matches!(arg.kind, ExprKind::Ident(_))));
    if !valid {
        return;
    }

    let ExprKind::Call { mut args, .. } = std::mem::replace(&mut expr.kind, ExprKind::Error) else {
        unreachable!();
    };
    let mut body = args.pop().expect("validated lets has a result expression");
    let let_symbol = Symbol {
        text: "let".to_string(),
    };
    // Move original argument nodes rather than reparsing or cloning source: spans
    // and property-reference identities survive the nested scope transformation.
    while let Some(value) = args.pop() {
        let ident = args
            .pop()
            .expect("validated lets has complete binding pairs");
        body = Expr {
            id: super::next_synthetic_id(),
            span: Span {
                start: ident.span.start,
                end: body.span.end,
            },
            kind: ExprKind::Call {
                callee: let_symbol.clone(),
                args: vec![ident, value, body],
            },
        };
    }
    // Keep the source call's identity and full span on the outermost let.
    expr.kind = body.kind;
}

/// Recurse into all child expressions of `expr` and desugar them.
fn desugar_children(expr: &mut Expr) {
    match &mut expr.kind {
        ExprKind::Lit(_) | ExprKind::Ident(_) | ExprKind::Error => {}
        ExprKind::Group { inner } => desugar_member_calls(inner),
        ExprKind::List { items } => {
            for item in items {
                desugar_member_calls(item);
            }
        }
        ExprKind::Unary { expr, .. } => desugar_member_calls(expr),
        ExprKind::Binary { left, right, .. } => {
            desugar_member_calls(left);
            desugar_member_calls(right);
        }
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            desugar_member_calls(cond);
            desugar_member_calls(then);
            desugar_member_calls(otherwise);
        }
        ExprKind::Call { args, .. } => {
            for arg in args {
                desugar_member_calls(arg);
            }
        }
        ExprKind::MemberCall { receiver, args, .. } => {
            desugar_member_calls(receiver);
            for arg in args {
                desugar_member_calls(arg);
            }
        }
        ExprKind::ImplicitLambda { body, .. } => desugar_member_calls(body),
    }
}
