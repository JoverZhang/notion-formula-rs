use crate::ast::{BinOpKind, Expr, ExprKind, UnOpKind};
use crate::parser::{infix_binding_power, prefix_binding_power};
use crate::token::{LitKind};

impl Expr {
    pub fn pretty(&self) -> String {
        self.pretty_with_prec(0)
    }

    fn pretty_with_prec(&self, parent_prec: u8) -> String {
        match &self.kind {
            ExprKind::Ident(sym) => sym.text.clone(),
            ExprKind::Lit(lit) => match lit.kind {
                LitKind::Number => lit.symbol.text.clone(),
                LitKind::String => escape_string_for_pretty(&lit.symbol.text),
                LitKind::Bool => lit.symbol.text.clone(),
            },
            ExprKind::Call { callee, args } => {
                let mut s = String::new();
                s.push_str(&callee.text);
                s.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&a.pretty_with_prec(0));
                }
                s.push(')');
                s
            }
            ExprKind::Unary { op, expr } => {
                let op_str = match op.node {
                    UnOpKind::Not => "!",
                    UnOpKind::Neg => "-",
                };
                let inner = expr.pretty_with_prec(prefix_binding_power(op.node));
                format!("{}{}", op_str, inner)
            }
            ExprKind::Binary { op, left, right } => {
                let (l_bp, r_bp) = infix_binding_power(op.node);
                let this_prec = l_bp;

                let l = left.pretty_with_prec(l_bp);
                let r = right.pretty_with_prec(r_bp);

                let op_str = binop_str(op.node);
                let combined = format!("{} {} {}", l, op_str, r);

                if this_prec < parent_prec {
                    format!("({})", combined)
                } else {
                    combined
                }
            }
            ExprKind::Error => "<error>".to_string(),
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
            } => {
                let cond = cond.pretty_with_prec(0);
                let then = then.pretty_with_prec(0);
                let otherwise = otherwise.pretty_with_prec(0);
                format!("{} ? {} : {}", cond, then, otherwise)
            }
        }
    }
}

fn binop_str(op: BinOpKind) -> &'static str {
    use BinOpKind::*;
    match op {
        Lt => "<",
        Le => "<=",
        EqEq => "==",
        Ne => "!=",
        Ge => ">=",
        Gt => ">",
        Dot => ".",
        AndAnd => "&&",
        OrOr => "||",
        Plus => "+",
        Minus => "-",
        Star => "*",
        Slash => "/",
        Percent => "%",
        Caret => "^",
    }
}

fn escape_string_for_pretty(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}
