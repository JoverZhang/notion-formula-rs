use crate::lexer::lex;
use crate::token::{Span, TokenKind, tokens_in_span};

#[test]
fn test_tokens_in_span() {
    // Exact boundary matches.
    let src = "(a+b)";
    let out = lex(src);
    assert!(out.diagnostics.is_empty());
    let tokens = out.tokens;

    let r = tokens_in_span(&tokens, Span { start: 1, end: 4 }); // "a+b"
    assert_eq!((r.lo, r.hi), (1, 4));

    let r = tokens_in_span(&tokens, Span { start: 1, end: 2 }); // "a"
    assert_eq!((r.lo, r.hi), (1, 2));
    assert!(matches!(tokens[r.lo as usize].kind, TokenKind::Ident(_)));

    let r = tokens_in_span(&tokens, Span { start: 2, end: 3 }); // "+"
    assert_eq!((r.lo, r.hi), (2, 3));
    assert!(matches!(tokens[r.lo as usize].kind, TokenKind::Plus));

    // Half-open end boundary is excluded.
    let r = tokens_in_span(&tokens, Span { start: 0, end: 2 }); // "(a", does not include '+'
    assert_eq!((r.lo, r.hi), (0, 2));
    assert!(matches!(tokens[0].kind, TokenKind::OpenParen));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));

    // Span that reaches end-of-input does not include EOF for non-empty spans.
    let len = src.len() as u32;
    let r = tokens_in_span(&tokens, Span { start: 0, end: len });
    assert!(matches!(tokens[r.hi as usize].kind, TokenKind::Eof));

    // Trivia inside the queried span is included (intersection-based).
    let src = "a/*c*/+b";
    let out = lex(src);
    assert!(out.diagnostics.is_empty());
    let tokens = out.tokens;

    let r = tokens_in_span(&tokens, Span { start: 0, end: 8 }); // whole expression
    assert_eq!((r.lo, r.hi), (0, 4)); // excludes EOF
    assert!(matches!(tokens[1].kind, TokenKind::BlockComment(_)));

    let r = tokens_in_span(&tokens, Span { start: 1, end: 6 }); // "/*c*/"
    assert_eq!((r.lo, r.hi), (1, 2));
    assert!(matches!(
        tokens[r.lo as usize].kind,
        TokenKind::BlockComment(_)
    ));

    // Span covering only trivia.
    let src = "a/*c*/b";
    let out = lex(src);
    assert!(out.diagnostics.is_empty());
    let tokens = out.tokens;
    let r = tokens_in_span(&tokens, Span { start: 1, end: 6 });
    assert_eq!((r.lo, r.hi), (1, 2));
    assert!(matches!(
        tokens[r.lo as usize].kind,
        TokenKind::BlockComment(_)
    ));

    // Span covering newline + comment + token.
    let src = "a\n/*c*/b";
    let out = lex(src);
    assert!(out.diagnostics.is_empty());
    let tokens = out.tokens;
    let r = tokens_in_span(&tokens, Span { start: 1, end: 8 });
    assert_eq!((r.lo, r.hi), (1, 4));
    assert!(matches!(tokens[1].kind, TokenKind::Newline));
    assert!(matches!(tokens[2].kind, TokenKind::BlockComment(_)));
    assert!(matches!(tokens[3].kind, TokenKind::Ident(_)));

    // Empty span: always empty result.
    let src = "(a+b)";
    let out = lex(src);
    assert!(out.diagnostics.is_empty());
    let tokens = out.tokens;

    let r = tokens_in_span(&tokens, Span { start: 2, end: 2 });
    assert_eq!((r.lo, r.hi), (2, 2));

    // Empty span at EOF: insertion point may be the EOF token index.
    let eof_idx = (tokens.len() - 1) as u32;
    let len = src.len() as u32;
    let r = tokens_in_span(
        &tokens,
        Span {
            start: len,
            end: len,
        },
    );
    assert_eq!((r.lo, r.hi), (eof_idx, eof_idx));

    // Out of bounds span: empty at the end (past EOF).
    let r = tokens_in_span(
        &tokens,
        Span {
            start: len + 100,
            end: len + 101,
        },
    );
    let end = tokens.len() as u32;
    assert_eq!((r.lo, r.hi), (end, end));
}
