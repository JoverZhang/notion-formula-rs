use std::mem::discriminant;

use crate::ast::{BinOp, BinOpKind, Expr, ExprKind};
use crate::token::{Lit, LitKind, NodeId, Span, Symbol, Token, TokenKind};

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

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn bump(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        self.pos += 1;
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        let tok = self.current().clone();
        if discriminant(&tok.kind) == discriminant(&kind) {
            self.pos += 1;
            Ok(tok)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: match kind {
                    TokenKind::Ident(symbol) => symbol.text,
                    TokenKind::Literal(lit) => lit.symbol.text,
                    TokenKind::Lt => "<".to_string(),
                    TokenKind::Le => "<=".to_string(),
                    TokenKind::EqEq => "==".to_string(),
                    TokenKind::Ne => "!=".to_string(),
                    TokenKind::Ge => ">=".to_string(),
                    TokenKind::Gt => ">".to_string(),
                    TokenKind::AndAnd => "&&".to_string(),
                    TokenKind::OrOr => "||".to_string(),
                    TokenKind::Bang => "!".to_string(),
                    TokenKind::Plus => "+".to_string(),
                    TokenKind::Minus => "-".to_string(),
                    TokenKind::Star => "*".to_string(),
                    TokenKind::Slash => "/".to_string(),
                    TokenKind::Percent => "%".to_string(),
                    TokenKind::Caret => "^".to_string(),
                    TokenKind::Dot => ".".to_string(),
                    TokenKind::Comma => ",".to_string(),
                    TokenKind::Colon => ":".to_string(),
                    TokenKind::Pound => "#".to_string(),
                    TokenKind::Question => "?".to_string(),
                    TokenKind::OpenParen => "(".to_string(),
                    TokenKind::CloseParen => ")".to_string(),
                    TokenKind::DocComment(..) => "# or /*".to_string(),
                    TokenKind::Eof => "EOF".to_string(),
                },
                found: tok.kind,
                span: tok.span,
            })
        }
    }
}

impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary_expr()
    }

    /// Only supports expressions like `primary` or `primary > primary`.
    fn parse_binary_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;

        if self.current().kind == TokenKind::Gt {
            let op_tok = self.bump().clone();
            let right = self.parse_primary()?;

            let span = Span {
                start: left.span.start,
                end: right.span.end,
            };

            left = Expr {
                id: self.alloc_id(),
                span,
                kind: ExprKind::Binary {
                    op: BinOp {
                        node: BinOpKind::Gt,
                        span: op_tok.span,
                    },
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match &self.current().kind {
            TokenKind::Ident(..) => self.parse_ident_or_call(),
            TokenKind::Literal(lit) => {
                match lit.kind {
                    LitKind::Number => self.parse_number_literal(),
                    LitKind::String => self.parse_string_literal(),
                    _ => {
                        let tok = self.current().clone();
                        Err(ParseError::UnexpectedToken {
                            expected: "number or string literal".to_string(),
                            found: tok.kind,
                            span: tok.span,
                        })
                    }
                }
            }
            _ => {
                let tok = self.current().clone();
                Err(ParseError::UnexpectedToken {
                    expected: "identifier or function call".to_string(),
                    found: tok.kind,
                    span: tok.span,
                })
            }
        }
    }

    fn parse_ident_or_call(&mut self) -> Result<Expr, ParseError> {
        let ident_tok = self.expect(TokenKind::Ident(Symbol { text: String::new() }))?;
        let ident_text = &self.source[ident_tok.span.start as usize..ident_tok.span.end as usize];

        // Here we simply handle: If the IDENT is followed by "(", it is treated as a function call.
        if self.current().kind == TokenKind::OpenParen {
            self.bump(); // consume "("

            // Currently only to support prop("Title") and subsequent possible multi-parameter calls.
            let mut args = Vec::new();
            // Simply: at least one parameter.
            args.push(self.parse_expr()?);

            while self.current().kind == TokenKind::Comma {
                self.bump(); // consume ","
                args.push(self.parse_expr()?);
            }

            let rparen = self.expect(TokenKind::CloseParen)?;

            let span = Span {
                start: ident_tok.span.start,
                end: rparen.span.end,
            };

            Ok(Expr {
                id: self.alloc_id(),
                span,
                kind: ExprKind::Call {
                    callee: ident_text.to_string(),
                    args,
                },
            })
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "identifier or function call".to_string(),
                found: self.current().kind.clone(),
                span: self.current().span,
            })
        }
    }

    fn parse_number_literal(&mut self) -> Result<Expr, ParseError> {
        let tok = self.expect(TokenKind::Literal(Lit { kind: LitKind::Number, symbol: Symbol { text: String::new() } }))?;
        let text = &self.source[tok.span.start as usize..tok.span.end as usize];
        let _value: f64 = text.parse().unwrap_or(0.0);

        Ok(Expr {
            id: self.alloc_id(),
            span: tok.span,
            kind: ExprKind::Lit(Lit {
                kind: LitKind::Number,
                symbol: Symbol { text: String::new() },
            }),
        })
    }

    fn parse_string_literal(&mut self) -> Result<Expr, ParseError> {
        let tok = self.expect(TokenKind::Literal(Lit { kind: LitKind::String, symbol: Symbol { text: String::new() } }))?;
        let text = &self.source[tok.span.start as usize..tok.span.end as usize];

        // Remove the leading and trailing quotes (without handling escapes).
        let inner = &text[1..text.len() - 1];

        Ok(Expr {
            id: self.alloc_id(),
            span: tok.span,
            kind: ExprKind::Lit(Lit {
                kind: LitKind::String,
                symbol: Symbol { text: inner.to_string() },
            }),
        })
    }
}
