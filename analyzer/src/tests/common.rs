pub fn trim_indent(s: &str) -> String {
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

macro_rules! assert_bin {
    ($e:expr, $op:pat) => {{
        match &($e).kind {
            ExprKind::Binary {
                op, left, right, ..
            } if matches!(op.node, $op) => (left.as_ref(), right.as_ref()),
            other => panic!("expected Binary({}), got {:?}", stringify!($op), other),
        }
    }};
}

macro_rules! assert_ternary {
    ($e:expr) => {{
        match &($e).kind {
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
                ..
            } => (cond.as_ref(), then.as_ref(), otherwise.as_ref()),
            other => panic!("expected Ternary, got {:?}", other),
        }
    }};
}

macro_rules! assert_lit_num {
    ($e:expr, $value:expr) => {{
        match &($e).kind {
            ExprKind::Lit(lit) if lit.kind == LitKind::Number => {
                assert_eq!(lit.symbol.text, $value.to_string());
            }
            other => panic!("expected Number literal, got {:?}", other),
        }
    }};
}

macro_rules! assert_lit_str {
    ($e:expr, $value:expr) => {{
        match &($e).kind {
            ExprKind::Lit(lit) if lit.kind == LitKind::String => {
                assert_eq!(lit.symbol.text, $value);
            }
            other => panic!("expected String literal, got {:?}", other),
        }
    }};
}

macro_rules! assert_call {
    ($e:expr, $callee:expr, $args:expr) => {{
        match &($e).kind {
            ExprKind::Call { callee, args, .. } => {
                assert_eq!(callee.text, $callee);
                assert_eq!(args.len(), $args);
                (callee, args)
            }
            other => panic!("expected Call, got {:?}", other),
        }
    }};
}

macro_rules! assert_list {
    ($e:expr, $items:expr) => {{
        match &($e).kind {
            ExprKind::List { items, .. } => {
                assert_eq!(items.len(), $items);
                items
            }
            other => panic!("expected List, got {:?}", other),
        }
    }};
}
