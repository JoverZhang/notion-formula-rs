use super::SignatureHelp;
use super::position::prev_non_trivia_before;
use crate::lexer::{Token, TokenKind};
use crate::semantic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CallContext {
    pub(super) callee: String,
    pub(super) lparen_idx: usize,
    pub(super) arg_index: usize,
}

pub(super) fn format_ty(ty: &semantic::Ty) -> String {
    match ty {
        semantic::Ty::Number => "number".into(),
        semantic::Ty::String => "string".into(),
        semantic::Ty::Boolean => "boolean".into(),
        semantic::Ty::Date => "date".into(),
        semantic::Ty::Null => "null".into(),
        semantic::Ty::Unknown => "unknown".into(),
        semantic::Ty::Generic(id) => format!("T{}", id.0),
        semantic::Ty::List(inner) => format!("{}[]", format_ty(inner)),
        semantic::Ty::Union(types) => types.iter().map(format_ty).collect::<Vec<_>>().join(" | "),
    }
}

fn format_param_sig(name: &str, param: &semantic::ParamSig) -> String {
    let mut ty = format_ty(&param.ty);
    if param.optional {
        ty.push('?');
    }
    format!("{name}: {ty}")
}

fn format_signature(sig: &semantic::FunctionSig) -> (String, Vec<String>) {
    if sig.params.repeat.is_empty() {
        let params = sig
            .params
            .head
            .iter()
            .chain(sig.params.tail.iter())
            .map(|p| format_param_sig(&p.name, p))
            .collect::<Vec<_>>();

        let label_params = params.join(", ");
        let label = format!("{}({}) -> {}", sig.name, label_params, format_ty(&sig.ret));
        return (label, params);
    }

    let mut params = Vec::<String>::new();
    params.extend(sig.params.head.iter().map(|p| format_param_sig(&p.name, p)));

    // Show the repeat pattern twice with numbering, then an ellipsis, then the tail.
    for n in 1..=2 {
        for p in &sig.params.repeat {
            let name = format!("{}{}", p.name, n);
            params.push(format_param_sig(&name, p));
        }
    }
    params.push("...".into());
    params.extend(sig.params.tail.iter().map(|p| format_param_sig(&p.name, p)));

    let label_params = params.join(", ");
    let label = format!("{}({}) -> {}", sig.name, label_params, format_ty(&sig.ret));
    (label, params)
}

/// Only compute signature help if the cursor is inside a function call argument context
/// (i.e., after the opening parenthesis).
pub(super) fn compute_signature_help_if_in_call(
    tokens: &[Token],
    cursor: u32,
    ctx: Option<&semantic::Context>,
    call_ctx: Option<&CallContext>,
) -> Option<SignatureHelp> {
    let call_ctx = call_ctx?;
    let lparen_token = tokens.get(call_ctx.lparen_idx)?;

    // Only show signature help if cursor is after the '(' (inside the call)
    if cursor < lparen_token.span.end {
        return None;
    }

    let ctx = ctx?;
    let func = ctx
        .functions
        .iter()
        .find(|func| func.name == call_ctx.callee)?;

    let is_postfix_call = || {
        let (callee_idx, callee_token) = prev_non_trivia_before(tokens, call_ctx.lparen_idx)?;
        let TokenKind::Ident(_) = callee_token.kind else {
            return None;
        };
        let (dot_idx, dot_token) = prev_non_trivia_before(tokens, callee_idx)?;
        if !matches!(dot_token.kind, TokenKind::Dot) {
            return None;
        }
        let (_, receiver_token) = prev_non_trivia_before(tokens, dot_idx)?;
        matches!(
            receiver_token.kind,
            TokenKind::Ident(_) | TokenKind::Literal(_) | TokenKind::CloseParen
        )
        .then_some(())
    };

    let is_postfix_call = is_postfix_call().is_some();

    if is_postfix_call && semantic::postfix_capable_builtin_names().contains(func.name.as_str()) {
        let params_all = func.flat_params()?;
        let receiver = params_all.first().map(|p| format_param_sig(&p.name, p));
        let params = params_all
            .iter()
            .skip(1)
            .map(|p| format_param_sig(&p.name, p))
            .collect::<Vec<_>>();

        let label_params = params.join(", ");
        let label = format!(
            "{}({}) -> {}",
            func.name,
            label_params,
            format_ty(&func.ret)
        );

        let active_param = if params.is_empty() {
            0
        } else {
            call_ctx.arg_index.min(params.len() - 1)
        };

        return Some(SignatureHelp {
            receiver,
            label,
            params,
            active_param,
        });
    }

    let (label, params) = format_signature(func);
    let active_param = if params.is_empty() {
        0
    } else {
        call_ctx.arg_index.min(params.len() - 1)
    };

    Some(SignatureHelp {
        receiver: None,
        label,
        params,
        active_param,
    })
}

pub(super) fn detect_call_context(tokens: &[Token], cursor: u32) -> Option<CallContext> {
    let mut stack = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        if token.span.start >= cursor {
            break;
        }
        match token.kind {
            TokenKind::OpenParen => stack.push(idx),
            TokenKind::CloseParen => {
                let _ = stack.pop();
            }
            _ => {}
        }
    }
    let lparen_idx = *stack.last()?;
    let (_, callee_token) = prev_non_trivia_before(tokens, lparen_idx)?;
    let TokenKind::Ident(ref symbol) = callee_token.kind else {
        return None;
    };
    let mut arg_index = 0usize;
    let mut depth = 0i32;
    for token in tokens.iter().skip(lparen_idx + 1) {
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        if token.span.start >= cursor {
            break;
        }
        match token.kind {
            TokenKind::OpenParen => depth += 1,
            TokenKind::CloseParen => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            TokenKind::Comma => {
                if depth == 0 {
                    arg_index += 1;
                }
            }
            _ => {}
        }
    }
    Some(CallContext {
        callee: symbol.text.clone(),
        lparen_idx,
        arg_index,
    })
}

pub(super) fn expected_call_arg_ty(
    call_ctx: Option<&CallContext>,
    ctx: Option<&semantic::Context>,
) -> Option<semantic::Ty> {
    let call_ctx = call_ctx?;
    let ctx = ctx?;
    ctx.functions
        .iter()
        .find(|func| func.name == call_ctx.callee)
        .and_then(|func| func.param_for_arg_index(call_ctx.arg_index))
        .map(|param| param.ty.clone())
}
