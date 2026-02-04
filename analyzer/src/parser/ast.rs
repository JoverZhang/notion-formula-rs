use crate::lexer::{Lit, NodeId, Span, Spanned, Symbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `==`
    EqEq,
    /// `!=`
    Ne,
    /// `>=`
    Ge,
    /// `>`
    Gt,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `^`
    Caret,
}

pub type BinOp = Spanned<BinOpKind>;

impl BinOp {
    /// Returns the Pratt binding power for an infix operator.
    ///
    /// Larger numbers bind tighter.
    ///
    /// Operator set handled by the expression parser:
    /// - Logical: `||`, `&&`
    /// - Equality: `==`, `!=`
    /// - Comparison: `<`, `<=`, `>=`, `>`
    /// - Arithmetic: `+`, `-`, `*`, `/`, `%`, `^`
    ///
    /// Associativity:
    /// - Most operators are left-associative (e.g. `a - b - c` parses as `(a - b) - c`).
    /// - `^` (power-like) is right-associative (e.g. `2 ^ 2 ^ 3` parses as `2 ^ (2 ^ 3)`).
    pub fn infix_binding_power(&self) -> (u8, u8) {
        use BinOpKind::*;

        // Return (left_bp, right_bp)
        // Right-associative: (p, p)
        // Left-associative: (p, p+1)
        // Here we use the classic Pratt parser:
        match self.node {
            OrOr => (3, 4),
            AndAnd => (5, 6),

            EqEq | Ne => (7, 8),
            Lt | Le | Ge | Gt => (9, 10),

            Plus | Minus => (11, 12),
            Star | Slash | Percent => (13, 14),
            Caret => (15, 15),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// `!`
    Not,
    /// `-`
    Neg,
}

impl UnOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnOp::Not => "!",
            UnOp::Neg => "-",
        }
    }

    /// Returns the Pratt binding power for a prefix operator.
    pub fn prefix_binding_power(&self) -> u8 {
        match self {
            UnOp::Not => 14,
            UnOp::Neg => 14,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub id: NodeId,
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Ident(Symbol),
    Group {
        inner: Box<Expr>,
    },
    List {
        items: Vec<Expr>,
    },
    Call {
        callee: Symbol,
        args: Vec<Expr>,
    },
    MemberCall {
        receiver: Box<Expr>,
        method: Symbol,
        args: Vec<Expr>,
    },
    Lit(Lit),
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Ternary {
        cond: Box<Expr>,
        then: Box<Expr>,
        otherwise: Box<Expr>,
    },
    Error,
}
