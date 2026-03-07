use crate::lexer::lex;
use crate::lexer::{CommentKind, Lit, LitKind, Span, Symbol, Token, TokenKind};

fn tokens(input: &str) -> Vec<Token> {
    lex(input).tokens
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

fn bool_lit(text: &str) -> TokenKind {
    TokenKind::Literal(Lit {
        kind: LitKind::Bool,
        symbol: Symbol {
            text: text.to_string(),
        },
    })
}

fn line_comment(text: &str) -> TokenKind {
    TokenKind::DocComment(
        CommentKind::Line,
        Symbol {
            text: text.to_string(),
        },
    )
}

fn block_comment(text: &str) -> TokenKind {
    TokenKind::DocComment(
        CommentKind::Block,
        Symbol {
            text: text.to_string(),
        },
    )
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
fn test_keywords_true_false_not() {
    let input = "true false not x";
    let expected = vec![
        bool_lit("true"),
        bool_lit("false"),
        TokenKind::Not,
        ident("x"),
        TokenKind::Eof,
    ];
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
        vec![
            TokenKind::Newline,
            number("1"),
            TokenKind::Plus,
            TokenKind::Newline,
            number("2"),
            TokenKind::Eof
        ]
    );

    let sp = spans(input);
    assert_eq!(sp[0], (2, 3)); // newline
    assert_eq!(sp[1], (4, 5));
    assert_eq!(sp[2], (8, 9));
    assert_eq!(sp[3], (9, 10)); // newline
    assert_eq!(sp[4], (10, 11));
    assert_eq!(sp[5], (12, 12));
}

#[test]
fn test_lex_error_single_equals() {
    let output = lex("=");
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0].message.contains("did you mean '=='"));
}

#[test]
fn test_lex_error_single_and() {
    let output = lex("&");
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0].message.contains("did you mean '&&'"));
}

#[test]
fn test_lex_error_single_or() {
    let output = lex("|");
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0].message.contains("did you mean '||'"));
}

#[test]
fn test_lex_error_unknown_char() {
    let output = lex("@");
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0].message.contains("unexpected char"));
    assert!(output.diagnostics[0].message.contains("@"));
    assert_eq!(output.diagnostics[0].span.start, 0);
    assert_eq!(output.diagnostics[0].span.end, 1);
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
    let output = lex("\"abc");
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0]
        .message
        .contains("unterminated string"));
}

#[test]
fn test_comment_tokens() {
    let input = "// guard\n1 /*mid*/ + /*mid2*/ 2\n3";
    let ks = kinds(input);
    assert_eq!(
        ks,
        vec![
            line_comment(" guard"),
            TokenKind::Newline,
            number("1"),
            block_comment("mid"),
            TokenKind::Plus,
            block_comment("mid2"),
            number("2"),
            TokenKind::Newline,
            number("3"),
            TokenKind::Eof
        ]
    );
}

#[test]
fn test_unterminated_string_recovers_partial_tokens() {
    let input = r#"if(prop("Number") > 10, prop("Text"), "Needs review)"#;
    let output = lex(input);
    assert!(!output.tokens.is_empty());
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0]
        .message
        .contains("unterminated string"));

    let has_prop = output
        .tokens
        .iter()
        .any(|tok| matches!(&tok.kind, TokenKind::Ident(sym) if sym.text == "prop"));
    let has_number = output.tokens.iter().any(|tok| {
        matches!(
            &tok.kind,
            TokenKind::Literal(Lit {
                kind: LitKind::String,
                symbol
            }) if symbol.text == r#""Number""#
        )
    });

    assert!(has_prop);
    assert!(has_number);
}

// ---------------------------------------------------------------------------
// Decimal and scientific notation numbers
// ---------------------------------------------------------------------------

#[test]
fn test_decimal_numbers() {
    let input = "3.14 0.5 100.0";
    let expected = vec![
        number("3.14"),
        number("0.5"),
        number("100.0"),
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_decimal_number_spans() {
    let input = "3.14";
    let sp = spans(input);
    assert_eq!(sp[0], (0, 4)); // "3.14"
}

#[test]
fn test_number_dot_ident_disambiguation() {
    // `3.method()` should lex as Number, Dot, Ident, OpenParen, CloseParen.
    let input = "3.method()";
    let expected = vec![
        number("3"),
        TokenKind::Dot,
        ident("method"),
        TokenKind::OpenParen,
        TokenKind::CloseParen,
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_number_dot_without_trailing_digits_is_int_plus_dot() {
    // `3.` followed by a non-digit: should be Number("3"), Dot.
    let input = "3. + 1";
    let expected = vec![
        number("3"),
        TokenKind::Dot,
        TokenKind::Plus,
        number("1"),
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_scientific_notation() {
    let input = "1e10 2E3";
    let expected = vec![number("1e10"), number("2E3"), TokenKind::Eof];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_scientific_notation_with_sign() {
    let input = "1e+2 3e-4 5E+6 7E-8";
    let expected = vec![
        number("1e+2"),
        number("3e-4"),
        number("5E+6"),
        number("7E-8"),
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_decimal_with_scientific_notation() {
    let input = "2.5e-3 1.0E10";
    let expected = vec![number("2.5e-3"), number("1.0E10"), TokenKind::Eof];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_number_e_without_digits_is_number_plus_ident() {
    // `1ex` should be Number("1"), Ident("ex") -- e not followed by digits.
    let input = "1ex";
    let expected = vec![number("1"), ident("ex"), TokenKind::Eof];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_number_e_sign_without_digits_is_separate_tokens() {
    // `1e+x` should be Number("1"), Ident("e"), Plus, Ident("x").
    let input = "1e+x";
    let expected = vec![
        number("1"),
        ident("e"),
        TokenKind::Plus,
        ident("x"),
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

// ---------------------------------------------------------------------------
// String escape sequences
// ---------------------------------------------------------------------------

#[test]
fn test_string_escape_sequences() {
    // The lexer stores the raw text including quotes and escape sequences.
    let input = r#""hello\nworld""#;
    let output = lex(input);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tokens[0].kind, string_lit(r#""hello\nworld""#));
}

#[test]
fn test_string_escaped_quote() {
    // `"say \"hi\""` -- escaped quotes inside a string.
    let input = r#""say \"hi\"""#;
    let output = lex(input);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tokens[0].kind, string_lit(r#""say \"hi\"""#));
    // Verify span covers entire string including the escaped quotes.
    assert_eq!(output.tokens[0].span.start, 0);
    assert_eq!(output.tokens[0].span.end, 12);
}

#[test]
fn test_string_escaped_backslash() {
    let input = r#""a\\b""#;
    let output = lex(input);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tokens[0].kind, string_lit(r#""a\\b""#));
}

#[test]
fn test_string_escaped_tab() {
    let input = r#""a\tb""#;
    let output = lex(input);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tokens[0].kind, string_lit(r#""a\tb""#));
}

#[test]
fn test_string_invalid_escape_emits_diagnostic() {
    let input = r#""a\xb""#;
    let output = lex(input);
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0].message.contains("invalid escape"));
    assert!(output.diagnostics[0].message.contains(r"\x"));
    // The string is still produced (with raw text).
    assert_eq!(output.tokens[0].kind, string_lit(r#""a\xb""#));
}

#[test]
fn test_string_unterminated_with_trailing_backslash() {
    let input = r#""abc\"#;
    let output = lex(input);
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0]
        .message
        .contains("unterminated string"));
}

// ---------------------------------------------------------------------------
// Identifier charset (Unicode letters, not emoji)
// ---------------------------------------------------------------------------

#[test]
fn test_identifier_cjk_accepted() {
    let input = "中文 日本語 한국어";
    let expected = vec![
        ident("中文"),
        ident("日本語"),
        ident("한국어"),
        TokenKind::Eof,
    ];
    assert_eq!(kinds(input), expected);
}

#[test]
fn test_identifier_emoji_rejected() {
    let input = "😀";
    let output = lex(input);
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0].message.contains("unexpected char"));
    assert!(output.diagnostics[0].message.contains("😀"));
}

#[test]
fn test_identifier_accented_accepted() {
    // Latin letters with accents are Unicode alphabetic.
    let input = "café naïve";
    let expected = vec![ident("café"), ident("naïve"), TokenKind::Eof];
    assert_eq!(kinds(input), expected);
}
