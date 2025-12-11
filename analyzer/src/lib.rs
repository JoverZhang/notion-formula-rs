use crate::{
    ast::Expr,
    lexer::lex,
    parser::{ParseError, Parser},
};

mod ast;
mod lexer;
mod parser;
mod token;

pub fn analyze(text: &str) -> Result<Expr, ParseError> {
    let tokens = lex(text).map_err(ParseError::LexError)?;
    let mut parser = Parser::new(text, tokens);
    let expr = parser.parse_expr()?;
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::ExprKind,
        token::{Lit, LitKind, Span, Symbol},
    };

    use super::*;

    #[test]
    fn test_analyze() {
        let ast = analyze(r#"if(prop("Title"), 1, 0)"#).unwrap();

        assert_eq!(
            Expr {
                id: 4,
                span: Span { start: 0, end: 23 },
                kind: ExprKind::Call {
                    callee: "if".to_string(),
                    args: vec![
                        Expr {
                            id: 1,
                            span: Span { start: 3, end: 16 },
                            kind: ExprKind::Call {
                                callee: "prop".to_string(),
                                args: vec![Expr {
                                    id: 0,
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
                            id: 2,
                            span: Span { start: 18, end: 19 },
                            kind: ExprKind::Lit(Lit {
                                kind: LitKind::Number,
                                symbol: Symbol {
                                    text: "".to_string(),
                                },
                            }),
                        },
                        Expr {
                            id: 3,
                            span: Span { start: 21, end: 22 },
                            kind: ExprKind::Lit(Lit {
                                kind: LitKind::Number,
                                symbol: Symbol {
                                    text: "".to_string(),
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
        let ast = analyze(&trim_indent(
            r#"
            if(
                prop("Title"),
                1,
                0
            )"#,
        ))
        .unwrap();

        assert_eq!(
            Expr {
                id: 4,
                span: Span { start: 0, end: 37 },
                kind: ExprKind::Call {
                    callee: "if".to_string(),
                    args: vec![
                        Expr {
                            id: 1,
                            span: Span { start: 8, end: 21 },
                            kind: ExprKind::Call {
                                callee: "prop".to_string(),
                                args: vec![Expr {
                                    id: 0,
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
                            id: 2,
                            span: Span { start: 27, end: 28 },
                            kind: ExprKind::Lit(Lit {
                                kind: LitKind::Number,
                                symbol: Symbol {
                                    text: "".to_string()
                                },
                            }),
                        },
                        Expr {
                            id: 3,
                            span: Span { start: 34, end: 35 },
                            kind: ExprKind::Lit(Lit {
                                kind: LitKind::Number,
                                symbol: Symbol {
                                    text: "".to_string()
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
    fn test_trim_indent() {
        let s = r#"
        if(
            prop("Title"),
            1,
            0
        )"#;
        let expected = "if(\n    prop(\"Title\"),\n    1,\n    0\n)";
        assert_eq!(expected, trim_indent(s));
    }

    fn trim_indent(s: &str) -> String {
        let lines: Vec<&str> = s.lines().collect();
        let min_indent = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
            .min()
            .unwrap_or(0);

        lines
            .iter()
            // Skip the first line (which is the empty line)
            .skip(1)
            .map(|l| {
                if l.len() >= min_indent {
                    &l[min_indent..]
                } else {
                    *l
                }
            })
            .collect::<Vec<&str>>()
            .join("\n")
    }
}
