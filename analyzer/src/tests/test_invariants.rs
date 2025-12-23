use crate::ast::{Expr, ExprKind};
use crate::lexer::lex;
use crate::parser::Parser;
use crate::token::{Span, Token, TokenRange};

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
    }
}

#[test]
fn test_span_and_tokenrange_invariants() {
    let cases = [
        "1+2*3",
        "(1+2)*3",
        "a&&b||c",
        "!a&&-b",
        r#"prop("Title",1+2*3)"#,
        "f(1,2,3)",
    ];

    for input in cases {
        let tokens = lex(input).unwrap();
        let mut parser = Parser::new(input, tokens.clone());
        let expr = parser.parse_expr().unwrap();
        check_invariants(&expr, &tokens);
    }
}
