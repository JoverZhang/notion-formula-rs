//! Pre-inference AST desugaring passes.
//!
//! Currently contains a single pass: [`desugar_member_calls`], which rewrites postfix
//! member-call syntax (`receiver.method(args...)`) into prefix call syntax
//! (`method(receiver, args...)`) for builtins that support it.
//!
//! This runs *before* type inference so that `infer_call` can mutate argument nodes
//! in-place (e.g. wrapping them in [`ExprKind::ImplicitLambda`]) without the mutations
//! being lost on a clone.

use crate::ast::{Expr, ExprKind};

/// Rewrite postfix member calls into prefix calls for postfix-capable builtins.
///
/// This mutates the AST in-place. After this pass, every `MemberCall` whose method is a
/// postfix-capable builtin has been replaced by an equivalent `Call` node with the
/// receiver prepended to the argument list.
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
