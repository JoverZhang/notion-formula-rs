use crate::{
    Token, TokenKind,
    lexer::{Lit, NodeId, Span, Spanned, Symbol},
};

pub enum AssocOp {
    Binary(BinOp),
    Ternary,
}

impl AssocOp {
    pub fn from_tok(tok: Token) -> Option<Self> {
        use AssocOp::*;

        if tok.kind == TokenKind::Question {
            Some(Ternary)
        } else {
            Some(Binary(BinOp::from_tok(tok)?))
        }
    }

    pub fn infix_binding_power(&self) -> (u8, u8) {
        use AssocOp::*;

        match self {
            Binary(op) => op.infix_binding_power(),
            // Lower precedence than `||` so `a || b ? c : d` parses as `(a || b) ? c : d`.
            // Right-associative: `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`.
            Ternary => (2, 1),
        }
    }
}

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
    pub fn from_tok(tok: Token) -> Option<Self> {
        let node = match tok.kind {
            TokenKind::Lt => BinOpKind::Lt,
            TokenKind::Le => BinOpKind::Le,
            TokenKind::EqEq => BinOpKind::EqEq,
            TokenKind::Ne => BinOpKind::Ne,
            TokenKind::Ge => BinOpKind::Ge,
            TokenKind::Gt => BinOpKind::Gt,
            TokenKind::AndAnd => BinOpKind::AndAnd,
            TokenKind::OrOr => BinOpKind::OrOr,
            TokenKind::Plus => BinOpKind::Plus,
            TokenKind::Minus => BinOpKind::Minus,
            TokenKind::Star => BinOpKind::Star,
            TokenKind::Slash => BinOpKind::Slash,
            TokenKind::Percent => BinOpKind::Percent,
            TokenKind::Caret => BinOpKind::Caret,
            _ => return None,
        };
        Some(Self {
            node,
            span: tok.span,
        })
    }

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
        // Left-associative: (p, p+1)
        // Right-associative: (p, p-1)
        // Here we use the classic Pratt parser:
        match self.node {
            // Logical OR
            OrOr => (3, 4),

            // Logical AND
            AndAnd => (5, 6),

            // Comparison
            EqEq | Ne => (7, 8),
            Lt | Le | Ge | Gt => (9, 10),

            // Addition
            Plus | Minus => (11, 12),
            Star | Slash | Percent => (13, 14),

            // Multiplication
            Caret => (16, 15),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotKind {
    /// `!`
    Bang,
    /// `not`
    Keyword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not(NotKind),
    /// `-`
    Neg,
}

impl UnOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnOp::Not(NotKind::Bang) => "!",
            UnOp::Not(NotKind::Keyword) => "not",
            UnOp::Neg => "-",
        }
    }

    /// Returns the Pratt binding power for a prefix operator.
    pub fn prefix_binding_power(&self) -> u8 {
        match self {
            UnOp::Not(_) => 14,
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
