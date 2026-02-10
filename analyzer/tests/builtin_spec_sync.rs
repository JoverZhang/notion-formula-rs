use analyzer::semantic::{FunctionCategory, builtins_functions};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct DocFunction {
    name: String,
    detail: String,
    category: FunctionCategory,
    status: Option<String>,
    line: usize,
}

#[test]
fn builtins_follow_builtin_functions_doc() {
    let doc = parse_doc_functions();
    assert!(
        !doc.is_empty(),
        "no signatures parsed from docs/builtin_functions/README.md"
    );

    let mut doc_by_name = HashMap::<String, DocFunction>::new();
    for f in &doc {
        let prev = doc_by_name.insert(f.name.clone(), f.clone());
        assert!(
            prev.is_none(),
            "duplicate function `{}` in docs at line {}",
            f.name,
            f.line
        );
    }

    let builtins = builtins_functions();
    let mut code_by_name = HashMap::new();
    for sig in &builtins {
        let prev = code_by_name.insert(sig.name.clone(), sig);
        assert!(prev.is_none(), "duplicate builtin function `{}`", sig.name);
    }

    for sig in &builtins {
        let spec = doc_by_name
            .get(&sig.name)
            .unwrap_or_else(|| panic!("builtin `{}` is not defined in docs", sig.name));
        assert!(
            spec.status.is_none(),
            "builtin `{}` is implemented, but docs mark it as `{}` (line {})",
            sig.name,
            spec.status.as_deref().unwrap_or("unknown"),
            spec.line
        );
        assert_eq!(
            sig.category, spec.category,
            "category mismatch for `{}` (doc line {})",
            sig.name, spec.line
        );
        assert_eq!(
            sig.detail, spec.detail,
            "detail mismatch for `{}` (doc line {})",
            sig.name, spec.line
        );
    }

    for spec in &doc {
        let implemented = code_by_name.contains_key(&spec.name);
        if spec.status.is_none() {
            assert!(
                implemented,
                "docs mark `{}` as implemented, but code has no builtin (line {})",
                spec.name, spec.line
            );
        } else {
            assert!(
                !implemented,
                "docs mark `{}` as `{}`, but code still implements it (line {})",
                spec.name,
                spec.status.as_deref().unwrap_or("unknown"),
                spec.line
            );
        }
    }

    let doc_order = doc
        .iter()
        .filter(|f| f.status.is_none())
        .map(|f| f.name.clone())
        .collect::<Vec<_>>();
    let code_order = builtins.iter().map(|f| f.name.clone()).collect::<Vec<_>>();
    assert_eq!(
        code_order, doc_order,
        "implemented builtin order must match docs"
    );
}

fn parse_doc_functions() -> Vec<DocFunction> {
    let path = doc_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let mut out = Vec::<DocFunction>::new();
    let mut category: Option<FunctionCategory> = None;
    let mut in_rust_block = false;
    let mut status: Option<String> = None;

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();

        if let Some(cat) = parse_category_header(line) {
            category = Some(cat);
            status = None;
            continue;
        }

        if line.starts_with("```rust") {
            in_rust_block = true;
            status = None;
            continue;
        }
        if line.starts_with("```") {
            in_rust_block = false;
            status = None;
            continue;
        }

        if !in_rust_block {
            continue;
        }

        if line.is_empty() {
            status = None;
            continue;
        }

        if line.starts_with("//") {
            if let Some(tag) = parse_todo_status(line) {
                status = Some(tag);
            }
            continue;
        }

        let Some((name, detail)) = parse_signature_line(line) else {
            continue;
        };
        let Some(cat) = category else {
            panic!("signature `{name}` appears outside category at line {line_no}");
        };

        out.push(DocFunction {
            name,
            detail,
            category: cat,
            status: status.clone(),
            line: line_no,
        });
    }

    out
}

fn parse_category_header(line: &str) -> Option<FunctionCategory> {
    let rest = line.strip_prefix("## ")?;
    let name = rest.split_whitespace().next()?;
    match name {
        "General" => Some(FunctionCategory::General),
        "Text" => Some(FunctionCategory::Text),
        "Number" => Some(FunctionCategory::Number),
        "Date" => Some(FunctionCategory::Date),
        "People" => Some(FunctionCategory::People),
        "List" => Some(FunctionCategory::List),
        "Special" => Some(FunctionCategory::Special),
        _ => None,
    }
}

fn parse_todo_status(line: &str) -> Option<String> {
    let start = line.find("TODO-")?;
    let tag = line[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();
    if tag.len() <= "TODO-".len() {
        return None;
    }
    Some(tag)
}

fn parse_signature_line(line: &str) -> Option<(String, String)> {
    let arrow = line.rfind("->")?;
    let lhs = line[..arrow].trim();

    let lparen = lhs.find('(')?;
    let rparen = lhs.rfind(')')?;
    if rparen <= lparen {
        return None;
    }

    let name_with_generics = lhs[..lparen].trim();
    let name = name_with_generics.split('<').next()?.trim();
    let first = name.chars().next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }

    let args = &lhs[lparen + 1..rparen];
    let args = split_top_level_commas(args)
        .into_iter()
        .filter_map(|arg| canonical_arg_name(&arg))
        .collect::<Vec<_>>()
        .join(", ");

    Some((name.to_string(), format!("{name}({args})")))
}

fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut start = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    let mut angle = 0usize;

    for (idx, ch) in s.char_indices() {
        match ch {
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            '{' => brace += 1,
            '}' => brace = brace.saturating_sub(1),
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            ',' if paren == 0 && bracket == 0 && brace == 0 && angle == 0 => {
                let part = s[start..idx].trim();
                if !part.is_empty() {
                    out.push(part.to_string());
                }
                start = idx + 1;
            }
            _ => {}
        }
    }

    let tail = s[start..].trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

fn canonical_arg_name(arg: &str) -> Option<String> {
    let arg = arg.trim();
    if arg.is_empty() {
        return None;
    }
    if arg == "..." {
        return Some("...".to_string());
    }
    if let Some(rest) = arg.strip_prefix("...") {
        let name = rest.split(':').next().unwrap_or(rest).trim();
        if name.is_empty() {
            return Some("...".to_string());
        }
        return Some(format!("...{name}"));
    }

    let name = arg.split(':').next().unwrap_or(arg).trim();
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

fn doc_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/builtin_functions/README.md")
}
