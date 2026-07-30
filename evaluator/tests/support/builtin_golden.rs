use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use analyzer::analysis::{Property, Ty};
use chrono::{DateTime, FixedOffset, SecondsFormat, TimeZone, Utc};
use evaluator::{
    AbiKind, AnyKind, BooleanKind, BuiltinRuntimeContext, Column, DateKind, EvalContext,
    EvalInputsBuilder, KernelColumn, ListKind, NumberKind, RowBatch, RowId, TextKind, Validity,
    Value, prepare_formula,
};
use serde_json::Value as JsonValue;

pub fn run_builtin_goldens(root: &Path) {
    let mut fixtures = Vec::new();
    collect_files_with_extension(root, "formula", &mut fixtures);
    fixtures.sort();
    assert!(
        !fixtures.is_empty(),
        "no builtin golden fixtures in {root:?}"
    );
    validate_catalog_coverage(root, &fixtures);

    for fixture in fixtures {
        run_fixture(&fixture);
    }
}

fn validate_catalog_coverage(root: &Path, fixtures: &[PathBuf]) {
    let actual = fixtures.iter().cloned().collect::<HashSet<_>>();
    let mut expected = HashSet::new();
    let mut missing = Vec::new();
    let mut supported_cases = HashSet::new();

    for category in builtin_fn::builtin_categories() {
        let directory = category_directory(category.category);
        for entry in category
            .entries
            .into_iter()
            .filter(|entry| entry.is_supported())
        {
            supported_cases.insert((directory, entry.name.clone()));
            let path = root.join(directory).join(format!("{}.formula", entry.name));
            if !actual.contains(&path) {
                missing.push(path.clone());
            }
            expected.insert(path);
        }
    }

    let mut invalid = Vec::new();
    for fixture in fixtures {
        let Some(directory) = fixture
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
        else {
            invalid.push(format!("{fixture:?}: missing category directory"));
            continue;
        };
        let Some(stem) = fixture.file_stem().and_then(|name| name.to_str()) else {
            invalid.push(format!("{fixture:?}: invalid fixture name"));
            continue;
        };
        let primary = stem.split_once("__").map_or(stem, |(name, _)| name);
        if !supported_cases.contains(&(directory, primary.to_string())) {
            invalid.push(format!(
                "{fixture:?}: `{primary}` is not a supported builtin in `{directory}`"
            ));
        }
    }

    assert!(
        missing.is_empty() && invalid.is_empty(),
        "builtin golden catalog mismatch\nmissing:\n{}\ninvalid:\n{}",
        missing
            .iter()
            .map(|path| format!("  {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n"),
        invalid
            .iter()
            .map(|message| format!("  {message}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(
        expected.len(),
        builtin_fn::builtins_functions().len(),
        "catalog contains duplicate supported builtin fixture paths"
    );

    let expected_snapshots = fixtures
        .iter()
        .map(|fixture| fixture.with_extension("snap"))
        .collect::<HashSet<_>>();
    let mut snapshots = Vec::new();
    collect_files_with_extension(root, "snap", &mut snapshots);
    let actual_snapshots = snapshots.into_iter().collect::<HashSet<_>>();
    let orphaned = actual_snapshots
        .difference(&expected_snapshots)
        .collect::<Vec<_>>();
    assert!(
        orphaned.is_empty(),
        "builtin golden directory contains snapshots without formulas: {orphaned:?}"
    );
}

fn category_directory(category: builtin_fn::FunctionCategory) -> &'static str {
    match category {
        builtin_fn::FunctionCategory::General => "general",
        builtin_fn::FunctionCategory::Text => "text",
        builtin_fn::FunctionCategory::Number => "number",
        builtin_fn::FunctionCategory::Date => "date",
        builtin_fn::FunctionCategory::People => "people",
        builtin_fn::FunctionCategory::List => "list",
        builtin_fn::FunctionCategory::Special => "special",
    }
}

fn collect_files_with_extension(directory: &Path, extension: &str, output: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read fixture directory {directory:?}: {error}"));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("failed to read entry in {directory:?}: {error}"))
            .path();
        if path.is_dir() {
            collect_files_with_extension(&path, extension, output);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            output.push(path);
        }
    }
}

fn run_fixture(path: &Path) {
    let source =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    validate_directives(path, &source);
    let properties = parse_properties(path, &source);
    let explicit_rows = parse_rows(path, &source);
    let explicit_mask = parse_mask(path, &source);
    let runtime = parse_runtime(path, &source);
    let batch_len = explicit_rows
        .as_ref()
        .map(Vec::len)
        .or_else(|| properties.first().map(|property| property.column.len()))
        .or_else(|| explicit_mask.as_ref().map(Vec::len))
        .unwrap_or(1);
    assert!(
        properties
            .iter()
            .all(|property| property.column.len() == batch_len),
        "{path:?}: property columns have different lengths"
    );
    let rows = explicit_rows.unwrap_or_else(|| {
        (0..batch_len)
            .map(|row| RowId::from(format!("row-{row}")))
            .collect()
    });
    let mask = explicit_mask
        .map(evaluator::Mask::from)
        .unwrap_or_else(|| evaluator::Mask::all(batch_len));
    assert_eq!(
        mask.len(),
        batch_len,
        "{path:?}: @mask length does not match batch length"
    );

    let mut syntax = analyzer::analyze_syntax(&source);
    assert!(
        syntax.diagnostics.is_empty(),
        "{path:?}: parse diagnostics: {:?}",
        syntax.diagnostics
    );
    let context = EvalContext::new(
        properties
            .iter()
            .map(|property| Property {
                name: property.name.clone(),
                ty: property.ty.clone(),
                disabled_reason: None,
            })
            .collect(),
    );
    let prepared = prepare_formula(&mut syntax.expr, &context)
        .unwrap_or_else(|error| panic!("{path:?}: failed to prepare fixture: {error:?}"));
    let primary = path
        .file_stem()
        .and_then(|name| name.to_str())
        .map(|stem| stem.split_once("__").map_or(stem, |(name, _)| name))
        .expect("catalog validation already checked fixture names");
    assert!(
        contains_call(&syntax.expr, primary),
        "{path:?}: fixture does not call its primary builtin `{primary}`"
    );
    let required_names = prepared
        .required_columns()
        .iter()
        .map(|required| required.name.as_str())
        .collect::<HashSet<_>>();
    for property in &properties {
        assert!(
            required_names.contains(property.name.as_str()),
            "{path:?}: property `{}` is declared but unused",
            property.name
        );
    }

    let mut inputs = EvalInputsBuilder::new(runtime.context.clone());
    for required in prepared.required_columns() {
        let property = properties
            .iter()
            .find(|property| property.name == required.name)
            .unwrap_or_else(|| panic!("{path:?}: missing property `{}`", required.name));
        inputs.insert(required.slot, property.column.clone());
    }
    let inputs = inputs
        .finish(&prepared, batch_len)
        .unwrap_or_else(|error| panic!("{path:?}: invalid evaluator inputs: {error:?}"));
    let output = prepared
        .evaluate_with_mask(RowBatch::new(rows.clone(), 7), inputs, mask.clone())
        .unwrap_or_else(|error| panic!("{path:?}: evaluation failed: {error:?}"));

    let actual = render_snapshot(&source, &rows, &mask, &output, runtime.offset);
    let snapshot_path = path.with_extension("snap");
    if std::env::var_os("BLESS").is_some() {
        fs::write(&snapshot_path, &actual)
            .unwrap_or_else(|error| panic!("failed to write {snapshot_path:?}: {error}"));
        return;
    }
    let expected = fs::read_to_string(&snapshot_path).unwrap_or_else(|_| {
        fs::write(&snapshot_path, &actual)
            .unwrap_or_else(|error| panic!("failed to write {snapshot_path:?}: {error}"));
        panic!(
            "generated missing golden file for {path:?}\ngolden: {snapshot_path:?}\n\
             review it, then rerun `cargo test -p evaluator --test builtin_golden`"
        );
    });
    assert_eq!(
        normalize(&expected),
        normalize(&actual),
        "golden mismatch\ninput: {path:?}\ngolden: {snapshot_path:?}"
    );
}

fn validate_directives(path: &Path, source: &str) {
    let mut formula_started = false;
    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        if line.starts_with("// @") {
            assert!(
                !formula_started,
                "{path:?}:{line_number}: fixture directives must precede the formula"
            );
            assert!(
                line.starts_with("// @prop ")
                    || line.starts_with("// @rows = ")
                    || line.starts_with("// @mask = ")
                    || line.starts_with("// @runtime = "),
                "{path:?}:{line_number}: unknown fixture directive `{line}`"
            );
        } else if !line.trim().is_empty() && !line.trim_start().starts_with("//") {
            formula_started = true;
        }
    }
}

fn contains_call(expression: &analyzer::ast::Expr, target: &str) -> bool {
    use analyzer::ast::ExprKind;

    match &expression.kind {
        ExprKind::Call { callee, args } => {
            callee.text == target || args.iter().any(|argument| contains_call(argument, target))
        }
        ExprKind::MemberCall {
            receiver,
            method,
            args,
        } => {
            method.text == target
                || contains_call(receiver, target)
                || args.iter().any(|argument| contains_call(argument, target))
        }
        ExprKind::Group { inner } | ExprKind::Unary { expr: inner, .. } => {
            contains_call(inner, target)
        }
        ExprKind::List { items } => items.iter().any(|item| contains_call(item, target)),
        ExprKind::Binary { left, right, .. } => {
            contains_call(left, target) || contains_call(right, target)
        }
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            contains_call(cond, target)
                || contains_call(then, target)
                || contains_call(otherwise, target)
        }
        ExprKind::ImplicitLambda { body, .. } => contains_call(body, target),
        ExprKind::Ident(_) | ExprKind::Lit(_) | ExprKind::Error => false,
    }
}

struct FixtureProperty {
    name: String,
    ty: Ty,
    column: Column,
}

struct FixtureRuntime {
    context: BuiltinRuntimeContext,
    offset: FixedOffset,
}

fn parse_runtime(path: &Path, source: &str) -> FixtureRuntime {
    let directives = source
        .lines()
        .filter_map(|line| line.strip_prefix("// @runtime = "))
        .collect::<Vec<_>>();
    assert!(
        directives.len() <= 1,
        "{path:?}: duplicate @runtime directive"
    );
    let value = directives
        .first()
        .map(|value| {
            serde_json::from_str::<String>(value)
                .unwrap_or_else(|error| panic!("{path:?}: invalid @runtime string: {error}"))
        })
        .unwrap_or_else(|| "2023-11-15T06:15:23.456+08:00".to_string());
    let date = DateTime::parse_from_rfc3339(&value)
        .unwrap_or_else(|error| panic!("{path:?}: invalid @runtime value: {error}"));
    let offset = *date.offset();
    FixtureRuntime {
        context: BuiltinRuntimeContext::new(date.timestamp_millis(), offset.local_minus_utc() / 60),
        offset,
    }
}

fn parse_rows(path: &Path, source: &str) -> Option<Vec<RowId>> {
    let directives = source
        .lines()
        .filter_map(|line| line.strip_prefix("// @rows = "))
        .collect::<Vec<_>>();
    assert!(directives.len() <= 1, "{path:?}: duplicate @rows directive");
    directives.first().map(|values| {
        serde_json::from_str::<Vec<String>>(values)
            .unwrap_or_else(|error| panic!("{path:?}: invalid @rows directive: {error}"))
            .into_iter()
            .map(RowId::from)
            .collect()
    })
}

fn parse_mask(path: &Path, source: &str) -> Option<Vec<bool>> {
    let directives = source
        .lines()
        .filter_map(|line| line.strip_prefix("// @mask = "))
        .collect::<Vec<_>>();
    assert!(directives.len() <= 1, "{path:?}: duplicate @mask directive");
    directives.first().map(|values| {
        serde_json::from_str::<Vec<bool>>(values)
            .unwrap_or_else(|error| panic!("{path:?}: invalid @mask directive: {error}"))
    })
}

fn parse_properties(path: &Path, source: &str) -> Vec<FixtureProperty> {
    let properties = source
        .lines()
        .filter_map(|line| line.strip_prefix("// @prop "))
        .map(|directive| {
            let (name, declaration) = split_property_name(directive)
                .unwrap_or_else(|| panic!("{path:?}: malformed @prop directive `{directive}`"));
            let name = serde_json::from_str::<String>(name)
                .unwrap_or_else(|error| panic!("{path:?}: invalid property name: {error}"));
            let (ty, values) = declaration
                .split_once(" = ")
                .unwrap_or_else(|| panic!("{path:?}: malformed @prop directive `{directive}`"));
            let values = serde_json::from_str::<Vec<JsonValue>>(values)
                .unwrap_or_else(|error| panic!("{path:?}: invalid property column: {error}"));
            let ty = parse_fixture_ty(ty)
                .unwrap_or_else(|| panic!("{path:?}: unsupported fixture property type `{ty}`"));
            let column = property_column(path, &name, &ty, &values);
            FixtureProperty { name, ty, column }
        })
        .collect::<Vec<_>>();
    let mut names = HashSet::new();
    for property in &properties {
        assert!(
            names.insert(property.name.as_str()),
            "{path:?}: duplicate property `{}`",
            property.name
        );
    }
    properties
}

fn split_property_name(directive: &str) -> Option<(&str, &str)> {
    if !directive.starts_with('"') {
        return None;
    }
    let mut escaped = false;
    for (index, character) in directive.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => {
                let declaration = directive[index + 1..].strip_prefix(": ")?;
                return Some((&directive[..=index], declaration));
            }
            _ => {}
        }
    }
    None
}

fn parse_fixture_ty(source: &str) -> Option<Ty> {
    let source = source.trim();
    if let Some(element) = source.strip_suffix("[]") {
        return parse_fixture_ty(strip_outer_parentheses(element))
            .map(|element| Ty::List(Box::new(element)));
    }
    let source = strip_outer_parentheses(source);
    let union = split_top_level_union(source);
    if union.len() > 1 {
        return union
            .into_iter()
            .map(parse_fixture_ty)
            .collect::<Option<Vec<_>>>()
            .map(builtin_fn::normalize_union);
    }
    match source {
        "number" => Some(Ty::Number),
        "string" => Some(Ty::String),
        "boolean" => Some(Ty::Boolean),
        "date" => Some(Ty::Date),
        "null" => Some(Ty::Null),
        "any" => Some(Ty::Unknown),
        _ => None,
    }
}

fn strip_outer_parentheses(source: &str) -> &str {
    let source = source.trim();
    if source.starts_with('(') && source.ends_with(')') {
        &source[1..source.len() - 1]
    } else {
        source
    }
}

fn split_top_level_union(source: &str) -> Vec<&str> {
    let mut depth = 0_u32;
    let mut start = 0;
    let mut members = Vec::new();
    for (index, character) in source.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => {
                members.push(source[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    members.push(source[start..].trim());
    members
}

fn property_column(path: &Path, name: &str, ty: &Ty, values: &[JsonValue]) -> Column {
    match evaluator::core::inputs::abi_kind_for_ty(ty) {
        AbiKind::Number => {
            let (values, validity) = typed_values(path, name, values, |value| {
                match json_to_typed_value(value, ty)? {
                    Value::Number(value) => Some(value),
                    _ => None,
                }
            });
            Column::Number(KernelColumn::<NumberKind>::from_values(values, validity))
        }
        AbiKind::Boolean => {
            let (values, validity) = typed_values(path, name, values, |value| {
                match json_to_typed_value(value, ty)? {
                    Value::Bool(value) => Some(value),
                    _ => None,
                }
            });
            Column::Boolean(KernelColumn::<BooleanKind>::from_values(values, validity))
        }
        AbiKind::Text => {
            let (values, validity) = typed_values(path, name, values, |value| {
                match json_to_typed_value(value, ty)? {
                    Value::Text(value) => Some(value),
                    _ => None,
                }
            });
            Column::Text(KernelColumn::<TextKind>::from_values(values, validity))
        }
        AbiKind::Date => {
            let (values, validity) = typed_values(path, name, values, |value| {
                match json_to_typed_value(value, ty)? {
                    Value::Date(value) => Some(value),
                    _ => None,
                }
            });
            Column::Date(KernelColumn::<DateKind>::from_values(values, validity))
        }
        AbiKind::List => {
            let (values, validity) = typed_values(path, name, values, |value| {
                match json_to_typed_value(value, ty)? {
                    Value::List(value) => Some(value),
                    _ => None,
                }
            });
            Column::List(KernelColumn::<ListKind>::from_values(values, validity))
        }
        AbiKind::Any => {
            let validity =
                Validity::from_valid_bits(values.iter().map(|value| !value.is_null()).collect());
            let values = values
                .iter()
                .enumerate()
                .map(|(row, value)| {
                    if value.is_null() {
                        Value::Number(0.0)
                    } else {
                        json_to_typed_value(value, ty).unwrap_or_else(|| {
                            panic!("{path:?}: property `{name}` has an invalid value at row {row}")
                        })
                    }
                })
                .collect();
            Column::Any(KernelColumn::<AnyKind>::from_values(values, validity))
        }
    }
}

fn json_to_typed_value(value: &JsonValue, ty: &Ty) -> Option<Value> {
    match ty {
        Ty::Number => value.as_f64().map(Value::Number),
        Ty::String => value.as_str().map(|value| Value::Text(value.to_owned())),
        Ty::Boolean => value.as_bool().map(Value::Bool),
        Ty::Date => DateTime::parse_from_rfc3339(value.as_str()?)
            .ok()
            .map(|date| Value::Date(date.timestamp_millis())),
        Ty::List(element) => value
            .as_array()?
            .iter()
            .map(|value| json_to_typed_value(value, element))
            .collect::<Option<Vec<_>>>()
            .map(Value::List),
        Ty::Union(members) => members
            .iter()
            .find_map(|member| json_to_typed_value(value, member)),
        Ty::Unknown => json_to_value(value),
        Ty::Null | Ty::Generic(_) | Ty::Fn { .. } | Ty::Ident(_) => None,
    }
}

fn json_to_value(value: &JsonValue) -> Option<Value> {
    match value {
        JsonValue::Bool(value) => Some(Value::Bool(*value)),
        JsonValue::Number(value) => value.as_f64().map(Value::Number),
        JsonValue::String(value) => Some(Value::Text(value.clone())),
        JsonValue::Array(values) => values
            .iter()
            .map(json_to_value)
            .collect::<Option<Vec<_>>>()
            .map(Value::List),
        JsonValue::Null | JsonValue::Object(_) => None,
    }
}

fn typed_values<T: Default>(
    path: &Path,
    name: &str,
    values: &[JsonValue],
    convert: impl Fn(&JsonValue) -> Option<T>,
) -> (Vec<T>, Validity) {
    let validity = Validity::from_valid_bits(
        values
            .iter()
            .map(|value| !value.is_null())
            .collect::<Vec<_>>(),
    );
    let values = values
        .iter()
        .enumerate()
        .map(|(row, value)| {
            if value.is_null() {
                T::default()
            } else {
                convert(value).unwrap_or_else(|| {
                    panic!("{path:?}: property `{name}` has an invalid value at row {row}")
                })
            }
        })
        .collect();
    (values, validity)
}

fn render_snapshot(
    source: &str,
    rows: &[RowId],
    mask: &evaluator::Mask,
    output: &evaluator::EvalBlock,
    offset: FixedOffset,
) -> String {
    let mut rendered = String::from("=== INPUT ===\n");
    rendered.push_str(source);
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered.push_str("=== OUTPUT ===\n");
    for (row, row_id) in rows.iter().enumerate() {
        rendered.push_str(row_id.as_str());
        rendered.push_str(": ");
        if !mask[row] {
            rendered.push_str("inactive");
        } else if !output.ok[row] {
            let errors = output
                .errors
                .iter()
                .filter(|(error_row, _)| *error_row == row)
                .map(|(_, error)| format!("{error:?}"))
                .collect::<Vec<_>>();
            assert_eq!(errors.len(), 1, "row {row_id} must have exactly one error");
            rendered.push_str("error(");
            rendered.push_str(&errors[0]);
            rendered.push(')');
        } else if let Some(value) = output.column.row_value(row) {
            render_value(&value, offset, &mut rendered);
        } else {
            rendered.push_str("null");
        }
        rendered.push('\n');
    }
    rendered
}

fn render_value(value: &Value, offset: FixedOffset, output: &mut String) {
    match value {
        Value::Number(value) => {
            if *value == 0.0 && value.is_sign_negative() {
                output.push_str("-0");
            } else {
                output.push_str(&value.to_string());
            }
        }
        Value::Text(value) => output.push_str(
            &serde_json::to_string(value).expect("serializing a Rust string cannot fail"),
        ),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Date(value) => {
            let date = Utc
                .timestamp_millis_opt(*value)
                .single()
                .unwrap_or_else(|| panic!("evaluator returned invalid date epoch {value}"))
                .with_timezone(&offset);
            output.push_str("date(");
            output.push_str(
                &serde_json::to_string(&date.to_rfc3339_opts(SecondsFormat::AutoSi, false))
                    .expect("serializing a date string cannot fail"),
            );
            output.push(')');
        }
        Value::List(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                render_value(value, offset, output);
            }
            output.push(']');
        }
    }
}

fn normalize(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n");
    if normalized.ends_with('\n') {
        normalized
    } else {
        format!("{normalized}\n")
    }
}
