use crate::ast::{BinOp, BinOpKind, Expr, ExprKind, UnOp, UnOpKind};
use crate::token::{Lit, LitKind, NodeId, Span, Symbol, Token, TokenKind, TokenRange};

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: TokenKind,
        span: Span,
    },
    LexError(String),
}

pub struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    next_id: NodeId,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Parser {
            source,
            tokens,
            pos: 0,
            next_id: 0,
        }
    }

    fn alloc_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn cur(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn cur_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn cur_idx(&self) -> u32 {
        self.pos as u32
    }

    fn bump(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    #[allow(unused)]
    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.same_kind(self.cur_kind(), &kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_punct(
        &mut self,
        kind: TokenKind,
        expected: &'static str,
    ) -> Result<Token, ParseError> {
        if self.same_kind(self.cur_kind(), &kind) {
            Ok(self.bump())
        } else {
            let tok = self.cur().clone();
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found: tok.kind,
                span: tok.span,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<Token, ParseError> {
        match self.cur_kind() {
            TokenKind::Ident(..) => Ok(self.bump()),
            _ => {
                let tok = self.cur().clone();
                Err(ParseError::UnexpectedToken {
                    expected: "identifier".to_string(),
                    found: tok.kind,
                    span: tok.span,
                })
            }
        }
    }

    fn expect_literal_kind(&mut self, k: LitKind) -> Result<Token, ParseError> {
        match self.cur_kind() {
            TokenKind::Literal(lit) if lit.kind == k => Ok(self.bump()),
            _ => {
                let tok = self.cur().clone();
                Err(ParseError::UnexpectedToken {
                    expected: format!("{:?} literal", k),
                    found: tok.kind,
                    span: tok.span,
                })
            }
        }
    }

    fn same_kind(&self, a: &TokenKind, b: &TokenKind) -> bool {
        use TokenKind::*;
        match (a, b) {
            (Ident(_), Ident(_)) => true,
            (Literal(_), Literal(_)) => true,
            (DocComment(..), DocComment(..)) => true,

            (Lt, Lt) | (Le, Le) | (EqEq, EqEq) | (Ne, Ne) | (Ge, Ge) | (Gt, Gt) => true,
            (AndAnd, AndAnd) | (OrOr, OrOr) | (Bang, Bang) => true,
            (Plus, Plus)
            | (Minus, Minus)
            | (Star, Star)
            | (Slash, Slash)
            | (Percent, Percent)
            | (Caret, Caret) => true,
            (Dot, Dot)
            | (Comma, Comma)
            | (Colon, Colon)
            | (Pound, Pound)
            | (Question, Question) => true,
            (OpenParen, OpenParen) | (CloseParen, CloseParen) | (Eof, Eof) => true,
            _ => false,
        }
    }

    fn span_from_tokens(&self, range: TokenRange) -> Span {
        let lo = range.lo as usize;
        let hi = range.hi as usize;
        if lo >= self.tokens.len() || hi == 0 || hi > self.tokens.len() || lo >= hi {
            // fallback: empty span
            return Span { start: 0, end: 0 };
        }
        let start = self.tokens[lo].span.start;
        let end = self.tokens[hi - 1].span.end;
        Span { start, end }
    }
}

// ---------- Pratt parser ----------
impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expr_bp(0)?;
        if !self.same_kind(self.cur_kind(), &TokenKind::Eof) {
            let tok = self.cur().clone();
            return Err(ParseError::UnexpectedToken {
                expected: "EOF".to_string(),
                found: tok.kind,
                span: tok.span,
            });
        }
        Ok(expr)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let op = match self.cur_kind() {
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
                _ => break,
            };

            let (l_bp, r_bp) = infix_binding_power(op);
            if l_bp < min_bp {
                break;
            }

            // consume op
            let op_tok_idx = self.cur_idx();
            let op_tok = self.bump();

            let rhs = self.parse_expr_bp(r_bp)?;

            let tokens = TokenRange::new(lhs.tokens.lo, rhs.tokens.hi);
            let span = self.span_from_tokens(tokens);

            lhs = Expr {
                id: self.alloc_id(),
                span,
                tokens,
                kind: ExprKind::Binary {
                    op: BinOp {
                        node: op,
                        span: op_tok.span,
                    },
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
            };

            // You might want to use op_tok_idx in the future for fixing the operator token location
            let _ = op_tok_idx;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        match self.cur_kind() {
            TokenKind::Bang => {
                let start = self.cur_idx();
                let tok = self.bump();
                let expr = self.parse_expr_bp(prefix_binding_power(UnOpKind::Not))?;
                let tokens = TokenRange::new(start, expr.tokens.hi);
                let span = self.span_from_tokens(tokens);

                Ok(Expr {
                    id: self.alloc_id(),
                    span,
                    tokens,
                    kind: ExprKind::Unary {
                        op: UnOp {
                            node: UnOpKind::Not,
                            span: tok.span,
                        },
                        expr: Box::new(expr),
                    },
                })
            }
            TokenKind::Minus => {
                let start = self.cur_idx();
                let tok = self.bump();
                let expr = self.parse_expr_bp(prefix_binding_power(UnOpKind::Neg))?;
                let tokens = TokenRange::new(start, expr.tokens.hi);
                let span = self.span_from_tokens(tokens);

                Ok(Expr {
                    id: self.alloc_id(),
                    span,
                    tokens,
                    kind: ExprKind::Unary {
                        op: UnOp {
                            node: UnOpKind::Neg,
                            span: tok.span,
                        },
                        expr: Box::new(expr),
                    },
                })
            }
            _ => self.parse_postfix_primary(),
        }
    }

    fn parse_postfix_primary(&mut self) -> Result<Expr, ParseError> {
        // primary: literal / ident / (expr)
        let mut expr = self.parse_primary()?;

        // postfix: call
        loop {
            if matches!(self.cur_kind(), TokenKind::OpenParen) {
                let lparen_idx = self.cur_idx();
                self.bump(); // consume '('

                let mut args = Vec::new();

                if !matches!(self.cur_kind(), TokenKind::CloseParen) {
                    args.push(self.parse_expr_bp(0)?);
                    while matches!(self.cur_kind(), TokenKind::Comma) {
                        self.bump(); // ','
                        args.push(self.parse_expr_bp(0)?);
                    }
                }

                self.expect_punct(TokenKind::CloseParen, "')'")?;

                let tokens = TokenRange::new(expr.tokens.lo, self.cur_idx()); // hi points after ')'
                let span = self.span_from_tokens(tokens);

                // Only Ident can call
                let callee = match expr.kind {
                    ExprKind::Ident(sym) => sym,
                    _ => {
                        return Err(ParseError::UnexpectedToken {
                            expected: "call callee (identifier)".to_string(),
                            found: self.cur().kind.clone(),
                            span: span,
                        });
                    }
                };

                expr = Expr {
                    id: self.alloc_id(),
                    span,
                    tokens,
                    kind: ExprKind::Call { callee, args },
                };

                let _ = lparen_idx;
                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.cur_kind() {
            TokenKind::Ident(_) => {
                let start = self.cur_idx();
                let tok = self.expect_ident()?;
                let tokens = TokenRange::new(start, start + 1);
                let span = tok.span;

                // The ident text can be directly used from the Symbol in tok.kind
                let sym = match tok.kind {
                    TokenKind::Ident(sym) => sym,
                    _ => unreachable!(),
                };

                Ok(Expr {
                    id: self.alloc_id(),
                    span,
                    tokens,
                    kind: ExprKind::Ident(sym),
                })
            }

            TokenKind::Literal(lit) => match lit.kind {
                LitKind::Number => self.parse_number_literal(),
                LitKind::String => self.parse_string_literal(),
                LitKind::Bool => {
                    // You lexer currently doesn't produce bool tokens (if you add true/false keywords in the future, go here)
                    let tok = self.bump();
                    let idx = (self.pos - 1) as u32;
                    Ok(Expr {
                        id: self.alloc_id(),
                        span: tok.span,
                        tokens: TokenRange::new(idx, idx + 1),
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Bool,
                            symbol: Symbol {
                                text: self.source[tok.span.start as usize..tok.span.end as usize]
                                    .to_string(),
                            },
                        }),
                    })
                }
            },

            TokenKind::OpenParen => {
                let start = self.cur_idx();
                self.bump(); // '('
                let mut inner = self.parse_expr_bp(0)?;
                self.expect_punct(TokenKind::CloseParen, "')'")?;

                // Wrap the parentheses token range around inner (without keeping the Group node)
                let tokens = TokenRange::new(start, self.cur_idx()); // hi points after ')'
                let span = self.span_from_tokens(tokens);
                inner.tokens = tokens;
                inner.span = span;
                Ok(inner)
            }

            _ => {
                let tok = self.cur().clone();
                Err(ParseError::UnexpectedToken {
                    expected: "primary expression".to_string(),
                    found: tok.kind,
                    span: tok.span,
                })
            }
        }
    }

    fn parse_number_literal(&mut self) -> Result<Expr, ParseError> {
        let start = self.cur_idx();
        let tok = self.expect_literal_kind(LitKind::Number)?;
        let tokens = TokenRange::new(start, start + 1);

        let text = &self.source[tok.span.start as usize..tok.span.end as usize];

        Ok(Expr {
            id: self.alloc_id(),
            span: tok.span,
            tokens,
            kind: ExprKind::Lit(Lit {
                kind: LitKind::Number,
                symbol: Symbol {
                    text: text.to_string(),
                },
            }),
        })
    }

    fn parse_string_literal(&mut self) -> Result<Expr, ParseError> {
        let start = self.cur_idx();
        let tok = self.expect_literal_kind(LitKind::String)?;
        let tokens = TokenRange::new(start, start + 1);

        let text = &self.source[tok.span.start as usize..tok.span.end as usize];
        let inner = if text.len() >= 2 {
            &text[1..text.len() - 1]
        } else {
            ""
        };

        Ok(Expr {
            id: self.alloc_id(),
            span: tok.span,
            tokens,
            kind: ExprKind::Lit(Lit {
                kind: LitKind::String,
                symbol: Symbol {
                    text: inner.to_string(),
                },
            }),
        })
    }
}

// precedence: bigger = tighter binding
fn infix_binding_power(op: BinOpKind) -> (u8, u8) {
    use BinOpKind::*;

    // Return (left_bp, right_bp)
    // Right-associative: (p, p) or (p, p-1)
    // Left-associative: (p, p+1)
    // Here we use the classic Pratt parser:
    match op {
        OrOr => (1, 2),
        AndAnd => (3, 4),
        EqEq | Ne => (5, 6),
        Lt | Le | Ge | Gt => (7, 8),
        Plus | Minus => (9, 10),
        Star | Slash | Percent => (11, 12),
        Caret => (13, 13),
        Dot => todo!(),
    }
}

fn prefix_binding_power(op: UnOpKind) -> u8 {
    match op {
        UnOpKind::Not => 14,
        UnOpKind::Neg => 14,
    }
}

// ---------- pretty print (single-line) ----------
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
            ExprKind::Ternary { cond, then, otherwise } => {
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
