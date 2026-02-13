use crate::ast::{BinOp, BinOpKind};
use crate::tests::common::trim_indent;
use crate::{
    analyze_syntax,
    ast::{Expr, ExprKind},
    lexer::{Lit, LitKind, Span, Symbol},
};

#[test]
fn test_analyze_single_line() {
    let parsed = analyze_syntax(r#"if(prop("Title"), 1, 0)"#);
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    assert_eq!(
        Expr {
            id: 6,
            span: Span { start: 0, end: 23 },
            kind: ExprKind::Call {
                callee: Symbol {
                    text: "if".to_string()
                },
                args: vec![
                    Expr {
                        id: 3,
                        span: Span { start: 3, end: 16 },
                        kind: ExprKind::Call {
                            callee: Symbol {
                                text: "prop".to_string()
                            },
                            args: vec![Expr {
                                id: 2,
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
    let parsed = analyze_syntax(&trim_indent(
        r#"
            if(
                prop("Title"),
                1,
                0
            )"#,
    ));
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    assert_eq!(
        Expr {
            id: 6,
            span: Span { start: 0, end: 37 },
            kind: ExprKind::Call {
                callee: Symbol {
                    text: "if".to_string()
                },
                args: vec![
                    Expr {
                        id: 3,
                        span: Span { start: 8, end: 21 },
                        kind: ExprKind::Call {
                            callee: Symbol {
                                text: "prop".to_string()
                            },
                            args: vec![Expr {
                                id: 2,
                                span: Span { start: 13, end: 20 },
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
    let parsed = analyze_syntax(r#"1 + 2 * 3"#);
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;
    assert_eq!(
        Expr {
            id: 4,
            span: Span { start: 0, end: 9 },
            kind: ExprKind::Binary {
                op: BinOp {
                    node: BinOpKind::Plus,
                    span: Span { start: 2, end: 3 }
                },
                left: Box::new(Expr {
                    id: 0,
                    span: Span { start: 0, end: 1 },
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
                    kind: ExprKind::Binary {
                        op: BinOp {
                            node: BinOpKind::Star,
                            span: Span { start: 6, end: 7 }
                        },
                        left: Box::new(Expr {
                            id: 1,
                            span: Span { start: 4, end: 5 },
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
