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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOpKind {
    Not,
    Neg,
}

pub type UnOp = Spanned<UnOpKind>;

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
