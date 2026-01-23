use crate::lexer::lex;
use crate::token::{tokens_in_span, Span, TokenKind};

#[test]
fn test_tokens_in_span() {
    // Exact boundary matches.
    let src = "(a+b)";
    let out = lex(src);
    assert!(out.diagnostics.is_empty());
    let tokens = out.tokens;

    let r = tokens_in_span(&tokens, Span { start: 1, end: 4 }); // "a+b"
    assert_eq!((r.lo, r.hi), (1, 4));

    let r = tokens_in_span(&tokens, Span { start: 2, end: 3 }); // "+"
    assert_eq!((r.lo, r.hi), (2, 3));
    assert!(matches!(tokens[r.lo as usize].kind, TokenKind::Plus));

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
    assert!(matches!(tokens[r.lo as usize].kind, TokenKind::BlockComment(_)));

    // Empty span: always empty result.
    let src = "(a+b)";
    let out = lex(src);
    assert!(out.diagnostics.is_empty());
    let tokens = out.tokens;

    let r = tokens_in_span(&tokens, Span { start: 2, end: 2 });
    assert_eq!((r.lo, r.hi), (2, 2));

    // Out of bounds span: empty at the end.
    let r = tokens_in_span(&tokens, Span { start: 100, end: 101 });
    let end = tokens.len() as u32;
    assert_eq!((r.lo, r.hi), (end, end));
}

