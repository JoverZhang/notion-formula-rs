use crate::lexer::lex;
use crate::token::{Lit, LitKind, Span, Symbol, Token, TokenKind};

fn tokens(input: &str) -> Vec<Token> {
    lex(input).unwrap()
}

fn kinds(input: &str) -> Vec<TokenKind> {
    tokens(input).into_iter().map(|t| t.kind).collect()
}

fn spans(input: &str) -> Vec<(u32, u32)> {
    tokens(input)
        .into_iter()
        .map(|t| (t.span.start, t.span.end))
        .collect()
}

fn ident(name: &str) -> TokenKind {
    TokenKind::Ident(Symbol {
        text: name.to_string(),
    })
}

fn number(text: &str) -> TokenKind {
    TokenKind::Literal(Lit {
        kind: LitKind::Number,
        symbol: Symbol {
            text: text.to_string(),
        },
    })
}

fn string_lit(text: &str) -> TokenKind {
    TokenKind::Literal(Lit {
        kind: LitKind::String,
        symbol: Symbol {
            text: text.to_string(),
        },
    })
}

#[test]
fn test_single_char_operators_and_punct() {
    let input = "< > ! + - * / % ^ . , : # ? ( )";
    let expected = vec![
        TokenKind::Lt,
        TokenKind::Gt,
        TokenKind::Bang,
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::Caret,
        TokenKind::Dot,
        TokenKind::Comma,
        TokenKind::Colon,
        TokenKind::Pound,
        TokenKind::Question,
        TokenKind::OpenParen,
        TokenKind::CloseParen,
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_multi_char_operators() {
    let input = "<= >= == != && ||";
    let expected = vec![
        TokenKind::Le,
        TokenKind::Ge,
        TokenKind::EqEq,
        TokenKind::Ne,
        TokenKind::AndAnd,
        TokenKind::OrOr,
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_mixed_expression_kinds() {
    let input = "a<=b && c!=d || e==f";
    let expected = vec![
        ident("a"),
        TokenKind::Le,
        ident("b"),
        TokenKind::AndAnd,
        ident("c"),
        TokenKind::Ne,
        ident("d"),
        TokenKind::OrOr,
        ident("e"),
        TokenKind::EqEq,
        ident("f"),
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_identifiers() {
    let input = "_a a1 A_B9";
    let expected = vec![ident("_a"), ident("a1"), ident("A_B9"), TokenKind::Eof];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_numbers() {
    let input = "0 12 345";
    let expected = vec![number("0"), number("12"), number("345"), TokenKind::Eof];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_string_literals() {
    let input = r#""a" "hello world""#;
    let toks = tokens(input);
    assert_eq!(toks.len(), 3);
    assert_eq!(toks[0].kind, string_lit(r#""a""#));
    assert_eq!(toks[1].kind, string_lit(r#""hello world""#));
    assert_eq!(toks[2].kind, TokenKind::Eof);
}

#[test]
fn test_whitespace_skipping_spans() {
    let input = "  \n\t1   +\n2\t";
    let ks = kinds(input);
    assert_eq!(
        ks,
        vec![number("1"), TokenKind::Plus, number("2"), TokenKind::Eof]
    );

    let sp = spans(input);
    assert_eq!(sp[0], (4, 5));
    assert_eq!(sp[1], (8, 9));
    assert_eq!(sp[2], (10, 11));
    assert_eq!(sp[3], (12, 12));
}

#[test]
fn test_lex_error_single_equals() {
    let err = lex("=").unwrap_err();
    assert!(err.contains("did you mean '=='"));
}

#[test]
fn test_lex_error_single_and() {
    let err = lex("&").unwrap_err();
    assert!(err.contains("did you mean '&&'"));
}

#[test]
fn test_lex_error_single_or() {
    let err = lex("|").unwrap_err();
    assert!(err.contains("did you mean '||'"));
}

#[test]
fn test_lex_error_unknown_char() {
    let err = lex("@").unwrap_err();
    assert!(err.contains("unexpected char"));
    assert!(err.contains("@"));
    assert!(err.contains("0"));
}

#[test]
fn test_empty_input_eof_span() {
    let toks = tokens("");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, TokenKind::Eof);
    assert_eq!(toks[0].span, Span { start: 0, end: 0 });
}

#[test]
fn test_unterminated_string_error() {
    let err = lex("\"abc").unwrap_err();
    assert!(err.contains("unterminated string"));
}
