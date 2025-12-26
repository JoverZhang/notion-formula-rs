pub type NodeId = u32;
pub type TokenIdx = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenRange {
    pub lo: TokenIdx, // inclusive
    pub hi: TokenIdx, // exclusive
}

impl TokenRange {
    pub fn new(lo: TokenIdx, hi: TokenIdx) -> Self {
        Self { lo, hi }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LitKind {
    Bool,
    Number,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lit {
    pub kind: LitKind,
    pub symbol: Symbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Line,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /* Expression-operator symbols. */
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
    /// `!`
    Bang,
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

    /* Structural symbols */
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `#`
    Pound,
    /// `?`
    Question,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,

    /* Literals */
    /// Literal token.
    Literal(Lit),
    /// Identifier token.
    Ident(Symbol),

    /// A doc comment token.
    /// `Symbol` is the doc comment's data excluding its "quotes" (`/*`, `#`, etc)
    /// similarly to symbols in string literal tokens.
    DocComment(CommentKind, Symbol),
    /// A line comment token.
    LineComment(Symbol),
    /// A block comment token.
    BlockComment(Symbol),
    /// Newline trivia (`\n`).
    Newline,

    /// End Of File
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    #[allow(unused)]
    pub fn can_begin_expr(&self) -> bool {
        match self.kind {
            TokenKind::Ident(..)
            | TokenKind::OpenParen
            | TokenKind::Literal(..)
            | TokenKind::Bang
            | TokenKind::Minus => true,
            _ => false,
        }
    }

    pub fn is_trivia(&self) -> bool {
        self.kind.is_trivia()
    }
}

impl TokenKind {
    pub fn is_trivia(&self) -> bool {
        matches!(
            self,
            TokenKind::LineComment(_)
                | TokenKind::BlockComment(_)
                | TokenKind::DocComment(..)
                | TokenKind::Newline
        )
    }

    pub fn is_comment(&self) -> bool {
        matches!(
            self,
            TokenKind::LineComment(_) | TokenKind::BlockComment(_) | TokenKind::DocComment(..)
        )
    }
}
