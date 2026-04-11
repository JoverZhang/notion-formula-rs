use super::{BuiltinSigParseError, BuiltinSigParseErrorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Ident(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Lt,
    Gt,
    Colon,
    Comma,
    Pipe,
    Question,
    Arrow,
    Ellipsis,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) position: usize,
}

impl TokenKind {
    pub(crate) fn display_name(&self) -> String {
        match self {
            TokenKind::Ident(name) => format!("identifier `{name}`"),
            TokenKind::LParen => "`(`".into(),
            TokenKind::RParen => "`)`".into(),
            TokenKind::LBracket => "`[`".into(),
            TokenKind::RBracket => "`]`".into(),
            TokenKind::Lt => "`<`".into(),
            TokenKind::Gt => "`>`".into(),
            TokenKind::Colon => "`:`".into(),
            TokenKind::Comma => "`,`".into(),
            TokenKind::Pipe => "`|`".into(),
            TokenKind::Question => "`?`".into(),
            TokenKind::Arrow => "`->`".into(),
            TokenKind::Ellipsis => "`...`".into(),
            TokenKind::Eof => "end of input".into(),
        }
    }
}

pub(crate) fn lex(input: &str) -> Result<Vec<Token>, BuiltinSigParseError> {
    let mut out = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some((idx, ch)) = chars.peek().copied() {
        if ch.is_ascii_whitespace() {
            chars.next();
            continue;
        }

        let token = match ch {
            '(' => {
                chars.next();
                Token {
                    kind: TokenKind::LParen,
                    position: idx,
                }
            }
            ')' => {
                chars.next();
                Token {
                    kind: TokenKind::RParen,
                    position: idx,
                }
            }
            '[' => {
                chars.next();
                Token {
                    kind: TokenKind::LBracket,
                    position: idx,
                }
            }
            ']' => {
                chars.next();
                Token {
                    kind: TokenKind::RBracket,
                    position: idx,
                }
            }
            '<' => {
                chars.next();
                Token {
                    kind: TokenKind::Lt,
                    position: idx,
                }
            }
            '>' => {
                chars.next();
                Token {
                    kind: TokenKind::Gt,
                    position: idx,
                }
            }
            ':' => {
                chars.next();
                Token {
                    kind: TokenKind::Colon,
                    position: idx,
                }
            }
            ',' => {
                chars.next();
                Token {
                    kind: TokenKind::Comma,
                    position: idx,
                }
            }
            '|' => {
                chars.next();
                Token {
                    kind: TokenKind::Pipe,
                    position: idx,
                }
            }
            '?' => {
                chars.next();
                Token {
                    kind: TokenKind::Question,
                    position: idx,
                }
            }
            '-' => {
                chars.next();
                match chars.next() {
                    Some((_, '>')) => Token {
                        kind: TokenKind::Arrow,
                        position: idx,
                    },
                    _ => {
                        return Err(BuiltinSigParseError::new(
                            BuiltinSigParseErrorKind::MissingArrow,
                            idx,
                        ));
                    }
                }
            }
            '.' => {
                chars.next();
                match (chars.next(), chars.next()) {
                    (Some((_, '.')), Some((_, '.'))) => Token {
                        kind: TokenKind::Ellipsis,
                        position: idx,
                    },
                    _ => {
                        return Err(BuiltinSigParseError::new(
                            BuiltinSigParseErrorKind::UnexpectedChar { ch },
                            idx,
                        ));
                    }
                }
            }
            _ if is_ident_start(ch) => {
                let mut end = idx + ch.len_utf8();
                chars.next();
                while let Some((next_idx, next_ch)) = chars.peek().copied() {
                    if is_ident_continue(next_ch) {
                        end = next_idx + next_ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                Token {
                    kind: TokenKind::Ident(input[idx..end].to_string()),
                    position: idx,
                }
            }
            _ => {
                return Err(BuiltinSigParseError::new(
                    BuiltinSigParseErrorKind::UnexpectedChar { ch },
                    idx,
                ));
            }
        };

        out.push(token);
    }

    out.push(Token {
        kind: TokenKind::Eof,
        position: input.len(),
    });
    Ok(out)
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
