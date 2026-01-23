use crate::ast::{Expr, ExprKind};
use crate::lexer::lex;
use crate::parser::Parser;
use crate::token::{Span, Token, tokens_in_span};
use crate::tokenstream::TokenCursor;

fn assert_child_range(child: &Expr, parent: &Expr, tokens: &[Token]) {
    let child_range = tokens_in_span(tokens, child.span.into());
    let parent_range = tokens_in_span(tokens, parent.span.into());
    assert!(
        child_range.lo >= parent_range.lo && child_range.hi <= parent_range.hi,
        "child range not within parent range"
    );
}

fn check_invariants(expr: &Expr, tokens: &[Token]) {
    let range = tokens_in_span(tokens, expr.span.into());
    assert!(range.lo < range.hi, "empty token range");

    let hi = range.hi as usize;
    assert!(hi <= tokens.len(), "token range out of bounds");

    let expected_span = Span {
        start: tokens[range.lo as usize].span.start,
        end: tokens[hi - 1].span.end,
    };
    assert_eq!(expr.span, expected_span);

    match &expr.kind {
        ExprKind::Group { inner } => {
            assert_child_range(inner, expr, tokens);
            check_invariants(inner, tokens);
        }
        ExprKind::Call { args, .. } => {
            for arg in args {
                assert_child_range(arg, expr, tokens);
                check_invariants(arg, tokens);
            }
        }
        ExprKind::MemberCall { receiver, args, .. } => {
            assert_child_range(receiver, expr, tokens);
            check_invariants(receiver, tokens);
            for arg in args {
                assert_child_range(arg, expr, tokens);
                check_invariants(arg, tokens);
            }
        }
        ExprKind::Unary { expr: inner, .. } => {
            assert_child_range(inner, expr, tokens);
            check_invariants(inner, tokens);
        }
        ExprKind::Binary { left, right, .. } => {
            assert_child_range(left, expr, tokens);
            assert_child_range(right, expr, tokens);
            check_invariants(left, tokens);
            check_invariants(right, tokens);
        }
        ExprKind::Ident(_) | ExprKind::Lit(_) | ExprKind::Error => {}
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            assert_child_range(cond, expr, tokens);
            assert_child_range(then, expr, tokens);
            assert_child_range(otherwise, expr, tokens);
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
