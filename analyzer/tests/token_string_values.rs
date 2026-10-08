use analyzer::{LitKind, Span, TokenKind, analyze_syntax, ast::ExprKind};

#[test]
fn string_tokens_expose_the_parser_value_without_changing_source_spelling_or_span() {
    for (source, expected) in [
        (r#""plain""#, "plain"),
        (r#""""#, ""),
        (
            r#""line\nnext\t\"quote\"\\path""#,
            "line\nnext\t\"quote\"\\path",
        ),
        ("\"中文😀\"", "中文😀"),
        ("\"raw\n\r\t\0\u{1}\"", "raw\n\r\t\0\u{1}"),
    ] {
        let parsed = analyze_syntax(source);
        assert!(parsed.diagnostics.is_empty(), "{source:?}");
        let token = &parsed.tokens[0];
        assert_eq!(
            token.string_value().as_deref(),
            Some(expected),
            "{source:?}"
        );
        assert_eq!(
            token.span,
            Span {
                start: 0,
                end: source.len() as u32
            }
        );
        let TokenKind::Literal(literal) = &token.kind else {
            panic!("expected string token");
        };
        assert_eq!(literal.kind, LitKind::String);
        assert_eq!(literal.symbol.text, source);
        let ExprKind::Lit(literal) = &parsed.expr.kind else {
            panic!("expected string expression");
        };
        assert_eq!(literal.symbol.text, expected);
    }
}

#[test]
fn invalid_escapes_have_no_token_value_and_keep_parser_recovery_and_diagnostics() {
    let source = r#""bad\q\n\x""#;
    let parsed = analyze_syntax(source);
    assert_eq!(parsed.tokens[0].string_value(), None);
    let TokenKind::Literal(literal) = &parsed.tokens[0].kind else {
        panic!("expected string token despite invalid escapes");
    };
    assert_eq!(literal.symbol.text, source);
    let ExprKind::Lit(literal) = &parsed.expr.kind else {
        panic!("expected recovered string expression");
    };
    assert_eq!(literal.symbol.text, "bad\\q\n\\x");
    let diagnostics: Vec<_> = parsed
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.message.as_str(), diagnostic.span))
        .collect();
    assert_eq!(
        diagnostics,
        [
            ("invalid escape sequence '\\q'", Span { start: 4, end: 6 }),
            ("invalid escape sequence '\\x'", Span { start: 8, end: 10 }),
        ]
    );
}

#[test]
fn nonstrings_and_unterminated_strings_do_not_expose_string_values() {
    let parsed = analyze_syntax("identifier + 1 + true");
    assert!(
        parsed
            .tokens
            .iter()
            .all(|token| token.string_value().is_none())
    );
    for source in ["\"unfinished", "\"trailing\\"] {
        let parsed = analyze_syntax(source);
        assert!(
            parsed
                .tokens
                .iter()
                .all(|token| token.string_value().is_none())
        );
        assert!(parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.message == "unterminated string literal"
                && diagnostic.span
                    == Span {
                        start: 0,
                        end: source.len() as u32,
                    }
        }));
    }
}
