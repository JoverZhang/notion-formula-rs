use crate::token::{Lit, NodeId, Span, Spanned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOpKind {
    /// The `>` operator (greater than)
    Gt,
}

pub type BinOp = Spanned<BinOpKind>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub id: NodeId,
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    /// A function call.
    Call { callee: String, args: Vec<Expr> },
    /// A literal (e.g., `1`, `"foo"`).
    Lit(Lit),
    /// A binary operation (e.g., `a + b`, `a * b`).
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}
