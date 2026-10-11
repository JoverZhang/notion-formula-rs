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
    let ExprKind::Call { callee, args } = &mut expr.kind else {
        return;
    };
    if callee.text != "lets" {
        return;
    }

    // Leave malformed calls intact for the usual call diagnostics.
    let Some((_result, binding_arguments)) = args.split_last() else {
        return;
    };
    let has_bindings = !binding_arguments.is_empty();
    let has_complete_binding_pairs = binding_arguments.len() % 2 == 0;
    if !has_bindings || !has_complete_binding_pairs {
        return;
    }

    let binding_names_are_identifiers = binding_arguments.chunks_exact(2).all(|binding_pair| {
        let binding_name = &binding_pair[0];
        matches!(&binding_name.kind, ExprKind::Ident(_))
    });
    if !binding_names_are_identifiers {
        return;
    }

    // Move original argument nodes to preserve their spans and property references.
    let mut remaining_arguments = std::mem::take(args);
    let mut body = remaining_arguments
        .pop()
        .expect("validated lets has a result expression");
    let let_symbol = Symbol {
        text: "let".to_string(),
    };

    // Wrap the result from the last binding back to the first.
    while let Some(binding_value) = remaining_arguments.pop() {
        let binding_name = remaining_arguments
            .pop()
            .expect("validated lets has complete binding pairs");
        body = Expr {
            id: super::next_synthetic_id(),
            span: Span {
                start: binding_name.span.start,
                end: body.span.end,
            },
            kind: ExprKind::Call {
                callee: let_symbol.clone(),
                args: vec![binding_name, binding_value, body],
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
