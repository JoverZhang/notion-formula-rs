use crate::token::{Lit, LitKind, Span, Symbol, Token, TokenKind};

pub fn lex(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut iter = input.char_indices().peekable();

    while let Some((start, ch)) = iter.next() {
        // Skip whitespace.
        if ch.is_whitespace() {
            continue;
        }

        let kind = match ch {
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            '!' => TokenKind::Bang,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '^' => TokenKind::Caret,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '#' => TokenKind::Pound,
            '?' => TokenKind::Question,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,

            '"' => {
                // Read string until the next ".
                let mut end = start + ch.len_utf8();
                while let Some((i, c)) = iter.next() {
                    if c == '"' {
                        end = i + 1;
                        break;
                    }
                }

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
                // Simple number literal (only supports integers).
                let mut end = start + 1;
                while let Some((&(i, c), _)) = iter.peek().map(|x| (x, ())) {
                    if c.is_ascii_digit() {
                        iter.next();
                        end = i + 1;
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

                while let Some((&(i, ch), _)) = iter.peek().map(|x| (x, ())) {
                    if is_ident_continue(ch) {
                        ident.push(ch);
                        iter.next();
                        end = i + ch.len_utf8();
                    } else {
                        break;
                    }
                }

                tokens.push(Token {
                    kind: TokenKind::Ident(Symbol {
                        text: ident,
                    }),
                    span: Span {
                        start: start as u32,
                        end: end as u32,
                    },
                });
                continue;
            }

            _ => {
                return Err(format!("unexpected char '{}' at {}", ch, start));
            }
        };

        tokens.push(Token {
            kind,
            span: Span {
                start: start as u32,
                end: (start + ch.len_utf8()) as u32,
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

    Ok(tokens)
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
