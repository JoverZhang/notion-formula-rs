use crate::diagnostics::{Diagnostic, DiagnosticKind};

mod token;

pub use token::{
    CommentKind, Lit, LitKind, NodeId, Span, Spanned, Symbol, Token, TokenIdx, TokenKind,
    TokenRange, tokens_in_span,
};

pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Lex the input into tokens.
///
/// - Numbers: ASCII digits only (no decimals).
/// - Strings: double-quoted, no escapes.
/// - Identifiers: ASCII letters/`_` and any non-ASCII codepoint.
pub fn lex(input: &str) -> LexOutput {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut iter = input.char_indices().peekable();

    while let Some((start, ch)) = iter.next() {
        // Skip spaces/tabs but keep newlines as trivia tokens.
        if matches!(ch, ' ' | '\t' | '\r') {
            continue;
        }

        if ch == '\n' {
            tokens.push(Token {
                kind: TokenKind::Newline,
                span: Span {
                    start: start as u32,
                    end: (start + 1) as u32,
                },
            });
            continue;
        }

        // Two-char operators first
        let kind = match ch {
            '#' => {
                if matches!(iter.peek(), Some((_, '#'))) {
                    let (_, _) = iter.next().unwrap();

                    let mut end = start + 2;
                    while let Some(&(i, c2)) = iter.peek() {
                        if c2 == '\n' {
                            break;
                        }
                        iter.next();
                        end = i + c2.len_utf8();
                    }

                    tokens.push(Token {
                        kind: TokenKind::DocComment(
                            CommentKind::Line,
                            Symbol {
                                text: String::from(&input[start + 2..end]),
                            },
                        ),
                        span: Span {
                            start: start as u32,
                            end: end as u32,
                        },
                    });
                    continue;
                } else {
                    TokenKind::Pound
                }
            }
            '<' => {
                if matches!(iter.peek(), Some((_, '='))) {
                    let (_, _) = iter.next().unwrap();
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if matches!(iter.peek(), Some((_, '='))) {
                    let (_, _) = iter.next().unwrap();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            '=' => {
                if matches!(iter.peek(), Some((_, '='))) {
                    let (_, _) = iter.next().unwrap();
                    TokenKind::EqEq
                } else {
                    diagnostics.push(make_error(
                        Span {
                            start: start as u32,
                            end: (start + 1) as u32,
                        },
                        "unexpected char '=' (did you mean '==')".to_string(),
                    ));
                    break;
                }
            }
            '!' => {
                if matches!(iter.peek(), Some((_, '='))) {
                    let (_, _) = iter.next().unwrap();
                    TokenKind::Ne
                } else {
                    TokenKind::Bang
                }
            }
            '&' => {
                if matches!(iter.peek(), Some((_, '&'))) {
                    let (_, _) = iter.next().unwrap();
                    TokenKind::AndAnd
                } else {
                    diagnostics.push(make_error(
                        Span {
                            start: start as u32,
                            end: (start + 1) as u32,
                        },
                        "unexpected char '&' (did you mean '&&')".to_string(),
                    ));
                    break;
                }
            }
            '|' => {
                if matches!(iter.peek(), Some((_, '|'))) {
                    let (_, _) = iter.next().unwrap();
                    TokenKind::OrOr
                } else {
                    diagnostics.push(make_error(
                        Span {
                            start: start as u32,
                            end: (start + 1) as u32,
                        },
                        "unexpected char '|' (did you mean '||')".to_string(),
                    ));
                    break;
                }
            }

            // one-char
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => {
                if matches!(iter.peek(), Some((_, '/'))) {
                    let (_, _) = iter.next().unwrap();
                    let mut end = start + 2;
                    while let Some(&(i, c2)) = iter.peek() {
                        if c2 == '\n' {
                            break;
                        }
                        iter.next();
                        end = i + c2.len_utf8();
                    }

                    tokens.push(Token {
                        kind: TokenKind::LineComment(Symbol {
                            text: String::from(&input[start + 2..end]),
                        }),
                        span: Span {
                            start: start as u32,
                            end: end as u32,
                        },
                    });
                    continue;
                } else if matches!(iter.peek(), Some((_, '*'))) {
                    let (_, _) = iter.next().unwrap();

                    let mut end = start + 2;
                    let mut terminated = false;
                    while let Some((i, c2)) = iter.next() {
                        if c2 == '*' && matches!(iter.peek(), Some((_, '/'))) {
                            let (j, slash) = iter.next().unwrap();
                            debug_assert_eq!(slash, '/');
                            end = j + slash.len_utf8();
                            terminated = true;
                            break;
                        }
                        end = i + c2.len_utf8();
                    }

                    if !terminated {
                        diagnostics.push(make_error(
                            Span {
                                start: start as u32,
                                end: input.len() as u32,
                            },
                            "unterminated block comment".to_string(),
                        ));
                        break;
                    }

                    tokens.push(Token {
                        kind: TokenKind::BlockComment(Symbol {
                            text: String::from(&input[start + 2..end - 2]),
                        }),
                        span: Span {
                            start: start as u32,
                            end: end as u32,
                        },
                    });
                    continue;
                } else {
                    TokenKind::Slash
                }
            }
            '%' => TokenKind::Percent,
            '^' => TokenKind::Caret,

            '.' => TokenKind::Dot,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '?' => TokenKind::Question,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,

            '"' => {
                // Read string until next quote (no escapes in v1).
                let mut end: Option<usize> = None;
                for (i, c) in iter.by_ref() {
                    if c == '"' {
                        end = Some(i + 1);
                        break;
                    }
                }

                let end = match end {
                    Some(end) => end,
                    None => {
                        diagnostics.push(make_error(
                            Span {
                                start: start as u32,
                                end: input.len() as u32,
                            },
                            "unterminated string literal".to_string(),
                        ));
                        break;
                    }
                };

                tokens.push(Token {
                    kind: TokenKind::Literal(Lit {
                        kind: LitKind::String,
                        symbol: Symbol {
                            text: String::from(&input[start..end]),
                        },
                    }),
                    span: Span {
                        start: start as u32,
                        end: end as u32,
                    },
                });
                continue;
            }

            c if c.is_ascii_digit() => {
                // integer number literal (v1)
                let mut end = start + c.len_utf8();
                while let Some(&(i, c2)) = iter.peek() {
                    if c2.is_ascii_digit() {
                        iter.next();
                        end = i + c2.len_utf8();
                    } else {
                        break;
                    }
                }

                tokens.push(Token {
                    kind: TokenKind::Literal(Lit {
                        kind: LitKind::Number,
                        symbol: Symbol {
                            text: String::from(&input[start..end]),
                        },
                    }),
                    span: Span {
                        start: start as u32,
                        end: end as u32,
                    },
                });
                continue;
            }

            c if is_ident_start(c) => {
                let mut end = start + c.len_utf8();
                let mut ident = String::new();
                ident.push(c);

                while let Some(&(i, ch2)) = iter.peek() {
                    if is_ident_continue(ch2) {
                        ident.push(ch2);
                        iter.next();
                        end = i + ch2.len_utf8();
                    } else {
                        break;
                    }
                }

                let kind = match ident.as_str() {
                    // Reserved keywords.
                    "not" => TokenKind::Not,
                    "true" | "false" => TokenKind::Literal(Lit {
                        kind: LitKind::Bool,
                        symbol: Symbol { text: ident },
                    }),
                    _ => TokenKind::Ident(Symbol { text: ident }),
                };

                tokens.push(Token {
                    kind,
                    span: Span {
                        start: start as u32,
                        end: end as u32,
                    },
                });
                continue;
            }

            _ => {
                diagnostics.push(make_error(
                    Span {
                        start: start as u32,
                        end: (start + ch.len_utf8()) as u32,
                    },
                    format!("unexpected char '{}'", ch),
                ));
                break;
            }
        };

        // span end: if you consume a double-character operator, you need to extend the end
        let end = match kind {
            TokenKind::Le
            | TokenKind::Ge
            | TokenKind::EqEq
            | TokenKind::Ne
            | TokenKind::AndAnd
            | TokenKind::OrOr => (start + 2) as u32,
            _ => (start + ch.len_utf8()) as u32,
        };

        tokens.push(Token {
            kind,
            span: Span {
                start: start as u32,
                end,
            },
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span {
            start: input.len() as u32,
            end: input.len() as u32,
        },
    });

    LexOutput {
        tokens,
        diagnostics,
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic() || c.len_utf8() > 1
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric() || c.len_utf8() > 1
}

fn make_error(span: Span, message: String) -> Diagnostic {
    Diagnostic {
        kind: DiagnosticKind::Error,
        message,
        span,
        labels: vec![],
        notes: vec![],
    }
}
