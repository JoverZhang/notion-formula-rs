use crate::ast::{Expr, ExprKind};
use crate::lexer::lex;
use crate::parser::Parser;
use crate::token::{Span, Token, TokenRange};
use crate::tokenstream::TokenCursor;

fn span_from_tokens(tokens: &[Token], range: TokenRange) -> Span {
    let lo = range.lo as usize;
    let hi = range.hi as usize;
    Span {
        start: tokens[lo].span.start,
        end: tokens[hi - 1].span.end,
    }
}

fn assert_child_range(child: &Expr, parent: &Expr) {
    assert!(
        child.tokens.lo >= parent.tokens.lo && child.tokens.hi <= parent.tokens.hi,
        "child range not within parent range"
    );
}

fn check_invariants(expr: &Expr, tokens: &[Token]) {
    assert!(expr.tokens.lo < expr.tokens.hi, "empty token range");

    let hi = expr.tokens.hi as usize;
    assert!(hi <= tokens.len(), "token range out of bounds");

    let expected_span = span_from_tokens(tokens, expr.tokens);
    assert_eq!(expr.span, expected_span);

    match &expr.kind {
        ExprKind::Group { inner } => {
            assert_child_range(inner, expr);
            check_invariants(inner, tokens);
        }
        ExprKind::Call { args, .. } => {
            for arg in args {
                assert_child_range(arg, expr);
                check_invariants(arg, tokens);
            }
        }
        ExprKind::Unary { expr: inner, .. } => {
            assert_child_range(inner, expr);
            check_invariants(inner, tokens);
        }
        ExprKind::Binary { left, right, .. } => {
            assert_child_range(left, expr);
            assert_child_range(right, expr);
            check_invariants(left, tokens);
            check_invariants(right, tokens);
        }
        ExprKind::Ident(_) | ExprKind::Lit(_) | ExprKind::Error => {}
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            assert_child_range(cond, expr);
            assert_child_range(then, expr);
            assert_child_range(otherwise, expr);
            check_invariants(cond, tokens);
            check_invariants(then, tokens);
            check_invariants(otherwise, tokens);
        }
    }
}

#[test]
fn test_span_and_tokenrange_invariants() {
    let cases = [
        "1+2*3",
        "(1+2)*3",
        "a&&b||c",
        "!a&&-b",
        "1 ? 2 : 3",
        "1 ? 2 : 3 ? 4 : 5",
        r#"prop("Title",1+2*3)"#,
        "f(1,2,3)",
    ];

    for input in cases {
        let output = lex(input);
        assert!(
            output.diagnostics.is_empty(),
            "expected no lex errors for {input:?}, got {:?}",
            output.diagnostics
        );
        let tokens = output.tokens;
        let token_cursor = TokenCursor::new(input, tokens.clone());
        let mut parser = Parser::new(token_cursor);
        let output = parser.parse_expr();
        assert!(
            output.diagnostics.is_empty(),
            "expected no parse errors for {input:?}, got {:?}",
            output.diagnostics
        );
        check_invariants(&output.expr, &tokens);
    }
}
