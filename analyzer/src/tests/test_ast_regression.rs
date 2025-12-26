use crate::ast::{BinOp, BinOpKind};
use crate::tests::common::trim_indent;
use crate::token::TokenRange;
use crate::{
    analyze,
    ast::{Expr, ExprKind},
    token::{Lit, LitKind, Span, Symbol},
};

#[test]
fn test_analyze_single_line() {
    let parsed = analyze(r#"if(prop("Title"), 1, 0)"#).unwrap();
    assert!(parsed.errors.is_empty());
    let ast = parsed.expr;

    assert_eq!(
        Expr {
            id: 6,
            tokens: TokenRange { lo: 0, hi: 11 },
            span: Span { start: 0, end: 23 },
            kind: ExprKind::Call {
                callee: Symbol {
                    text: "if".to_string()
                },
                args: vec![
                    Expr {
                        id: 3,
                        span: Span { start: 3, end: 16 },
                        tokens: TokenRange { lo: 2, hi: 6 },
                        kind: ExprKind::Call {
                            callee: Symbol {
                                text: "prop".to_string()
                            },
                            args: vec![Expr {
                                id: 2,
                                tokens: TokenRange { lo: 4, hi: 5 },
                                span: Span { start: 8, end: 15 },
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::String,
                                    symbol: Symbol {
                                        text: "Title".to_string(),
                                    },
                                }),
                            }],
                        },
                    },
                    Expr {
                        id: 4,
                        span: Span { start: 18, end: 19 },
                        tokens: TokenRange { lo: 7, hi: 8 },
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Number,
                            symbol: Symbol {
                                text: "1".to_string(),
                            },
                        }),
                    },
                    Expr {
                        id: 5,
                        span: Span { start: 21, end: 22 },
                        tokens: TokenRange { lo: 9, hi: 10 },
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Number,
                            symbol: Symbol {
                                text: "0".to_string(),
                            },
                        }),
                    },
                ],
            },
        },
        ast,
    );
}

#[test]
fn test_analyze_multiple_lines() {
    let parsed = analyze(&trim_indent(
        r#"
            if(
                prop("Title"),
                1,
                0
            )"#,
    ))
    .unwrap();
    assert!(parsed.errors.is_empty());
    let ast = parsed.expr;

    assert_eq!(
        Expr {
            id: 6,
            span: Span { start: 0, end: 37 },
            tokens: TokenRange { lo: 0, hi: 11 },
            kind: ExprKind::Call {
                callee: Symbol {
                    text: "if".to_string()
                },
                args: vec![
                    Expr {
                        id: 3,
                        span: Span { start: 8, end: 21 },
                        tokens: TokenRange { lo: 2, hi: 6 },
                        kind: ExprKind::Call {
                            callee: Symbol {
                                text: "prop".to_string()
                            },
                            args: vec![Expr {
                                id: 2,
                                span: Span { start: 13, end: 20 },
                                tokens: TokenRange { lo: 4, hi: 5 },
                                kind: ExprKind::Lit(Lit {
                                    kind: LitKind::String,
                                    symbol: Symbol {
                                        text: "Title".to_string()
                                    }
                                }),
                            }],
                        },
                    },
                    Expr {
                        id: 4,
                        span: Span { start: 27, end: 28 },
                        tokens: TokenRange { lo: 7, hi: 8 },
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Number,
                            symbol: Symbol {
                                text: "1".to_string()
                            },
                        }),
                    },
                    Expr {
                        id: 5,
                        span: Span { start: 34, end: 35 },
                        tokens: TokenRange { lo: 9, hi: 10 },
                        kind: ExprKind::Lit(Lit {
                            kind: LitKind::Number,
                            symbol: Symbol {
                                text: "0".to_string()
                            },
                        }),
                    }
                ]
            },
        },
        ast,
    );
}

#[test]
fn test_precedence() {
    let parsed = analyze(r#"1 + 2 * 3"#).unwrap();
    assert!(parsed.errors.is_empty());
    let ast = parsed.expr;
    assert_eq!(
        Expr {
            id: 4,
            span: Span { start: 0, end: 9 },
            tokens: TokenRange { lo: 0, hi: 5 },
            kind: ExprKind::Binary {
                op: BinOp {
                    node: BinOpKind::Plus,
                    span: Span { start: 2, end: 3 }
                },
                left: Box::new(Expr {
                    id: 0,
                    span: Span { start: 0, end: 1 },
                    tokens: TokenRange { lo: 0, hi: 1 },
                    kind: ExprKind::Lit(Lit {
                        kind: LitKind::Number,
                        symbol: Symbol {
                            text: "1".to_string()
                        }
                    })
                }),
                right: Box::new(Expr {
                    id: 3,
                    span: Span { start: 4, end: 9 },
                    tokens: TokenRange { lo: 2, hi: 5 },
                    kind: ExprKind::Binary {
                        op: BinOp {
                            node: BinOpKind::Star,
                            span: Span { start: 6, end: 7 }
                        },
                        left: Box::new(Expr {
                            id: 1,
                            span: Span { start: 4, end: 5 },
                            tokens: TokenRange { lo: 2, hi: 3 },
                            kind: ExprKind::Lit(Lit {
                                kind: LitKind::Number,
                                symbol: Symbol {
                                    text: "2".to_string()
                                }
                            })
                        }),
                        right: Box::new(Expr {
                            id: 2,
                            span: Span { start: 8, end: 9 },
                            tokens: TokenRange { lo: 4, hi: 5 },
                            kind: ExprKind::Lit(Lit {
                                kind: LitKind::Number,
                                symbol: Symbol {
                                    text: "3".to_string()
                                }
                            })
                        }),
                    }
                }),
            }
        },
        ast,
    );
}
