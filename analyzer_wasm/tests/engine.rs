#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;
use std::{cell::Cell, rc::Rc};

use analyzer_wasm::{FormulaEngineSession, dto::engine as dto};
use formula_engine as native;
use js_sys::{Array, Map, Reflect};
use serde::Serialize;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_test::wasm_bindgen_test;

fn value<T: Serialize>(input: &T) -> JsValue {
    input
        .serialize(
            &serde_wasm_bindgen::Serializer::new()
                .serialize_large_number_types_as_bigints(true)
                .serialize_missing_as_null(true),
        )
        .unwrap()
}

fn field(input: &JsValue, name: &str) -> JsValue {
    Reflect::get(input, &JsValue::from_str(name)).unwrap()
}

fn set(input: &JsValue, name: &str, replacement: &JsValue) {
    Reflect::set(input, &JsValue::from_str(name), replacement).unwrap();
}

fn array(input: &JsValue) -> Array {
    input.clone().dyn_into().unwrap()
}

fn error_code(error: &JsValue, code: &str) {
    assert_eq!(field(error, "code").as_string().as_deref(), Some(code));
    assert!(!field(error, "message").as_string().unwrap().is_empty());
    assert!(!field(error, "payload").is_undefined());
}

fn invalid_dto(error: &JsValue, operation: &str) {
    error_code(error, "INVALID_DTO");
    assert_eq!(
        field(&field(error, "payload"), "operation"),
        JsValue::from_str(operation)
    );
}

fn definition(id: &str, expression: &str) -> dto::FormulaDefinition {
    dto::FormulaDefinition {
        id: id.into(),
        expression: expression.into(),
    }
}

fn input(id: &str, ty: dto::ValueType) -> dto::PropertyDefinition {
    dto::PropertyDefinition::Input { id: id.into(), ty }
}

fn formula(id: &str, expression: &str) -> dto::PropertyDefinition {
    dto::PropertyDefinition::Formula(definition(id, expression))
}

fn session(properties: Vec<dto::PropertyDefinition>) -> FormulaEngineSession {
    FormulaEngineSession::new(value(&dto::FormulaSchema { properties })).unwrap()
}

fn empty_input(ids: &[&str]) -> dto::EvaluateInput {
    dto::EvaluateInput {
        row_ids: vec!["row".into()],
        columns: HashMap::new(),
        runtime: dto::RuntimeContext {
            now: 1_700_000_123_456,
            time_zone: "+08:00".into(),
        },
        formula_ids: ids.iter().map(|id| (*id).into()).collect(),
    }
}

fn output(result: &JsValue, id: &str) -> JsValue {
    let formulas: Map = field(result, "formulas").dyn_into().unwrap();
    field(&formulas.get(&JsValue::from_str(id)), "Ok")
}

#[wasm_bindgen_test]
fn draft_reference_spans_use_utf16_and_decoded_ids() {
    let mut engine = session(vec![input("字段🙂", dto::ValueType::Number)]);
    let source =
        r#"["中🙂", prop( /* preserved */ "字段🙂" ), prop("missing"), "prop(\"ignored\")"]"#;
    let handle = engine
        .create_draft(value(&definition("candidate", source)))
        .unwrap();
    let state = engine.draft_state(handle).unwrap();
    let references = array(&field(&state, "property_references"));
    assert_eq!(references.length(), 2);
    let first = references.get(0);
    assert_eq!(
        field(&first, "property_id").as_string().as_deref(),
        Some("字段🙂")
    );
    let expected_call = r#"prop( /* preserved */ "字段🙂" )"#;
    let expected_id = r#""字段🙂""#;
    for (key, text) in [("span", expected_call), ("id_span", expected_id)] {
        let start = source.find(text).unwrap();
        let span = field(&first, key);
        assert_eq!(
            field(&span, "start").as_f64(),
            Some(source[..start].encode_utf16().count() as f64)
        );
        assert_eq!(
            field(&span, "end").as_f64(),
            Some(source[..start + text.len()].encode_utf16().count() as f64)
        );
    }
    assert_eq!(
        field(&references.get(1), "property_id")
            .as_string()
            .as_deref(),
        Some("missing")
    );
    engine.draft_close(handle);
    assert_eq!(array(&field(&state, "property_references")).length(), 2);
}

#[wasm_bindgen_test]
fn strict_schema_and_formula_fields_reject_before_engine_mutations() {
    let malformed = value(&dto::FormulaSchema { properties: vec![] });
    set(&malformed, "extra", &JsValue::from_f64(1.0));
    let error = match FormulaEngineSession::new(malformed) {
        Err(error) => error,
        Ok(_) => panic!("unknown schema fields must fail"),
    };
    invalid_dto(&error, "new");

    let malformed = value(&dto::FormulaSchema {
        properties: vec![formula("saved", "1")],
    });
    let nested = field(&array(&field(&malformed, "properties")).get(0), "Formula");
    set(&nested, "extra", &JsValue::from_f64(1.0));
    let error = match FormulaEngineSession::new(malformed) {
        Err(error) => error,
        Ok(_) => panic!("unknown nested formula fields must fail"),
    };
    invalid_dto(&error, "new");

    let mut engine = session(vec![formula("saved", "1")]);
    let malformed = value(&definition("candidate", "2"));
    set(&malformed, "extra", &JsValue::NULL);
    invalid_dto(&engine.create_draft(malformed).unwrap_err(), "create_draft");
    let handle = engine
        .create_draft(value(&definition("candidate", "2")))
        .unwrap();
    assert_eq!(handle, 1, "invalid DTO must not allocate a draft handle");
    engine.draft_close(handle);

    let malformed = value(&formula("saved", "2"));
    set(&field(&malformed, "Formula"), "extra", &JsValue::UNDEFINED);
    invalid_dto(&engine.upsert(malformed).unwrap_err(), "upsert");
    let saved = engine.get_property("saved".into()).unwrap();
    assert_eq!(
        field(
            &field(&field(&saved, "Formula"), "definition"),
            "expression"
        ),
        JsValue::from_str("1")
    );
    engine.upsert(value(&formula("saved", "2"))).unwrap();
}

#[wasm_bindgen_test]
fn strict_evaluate_and_runtime_fields_reject_nested_unknown_fields() {
    let engine = session(vec![formula("ok", "1")]);
    let request = empty_input(&["ok"]);
    assert!(engine.evaluate(value(&request)).is_ok());
    let malformed = value(&request);
    set(&malformed, "extra", &JsValue::from_f64(1.0));
    invalid_dto(&engine.evaluate(malformed).unwrap_err(), "evaluate");
    let malformed = value(&request);
    set(
        &field(&malformed, "runtime"),
        "extra",
        &JsValue::from_f64(1.0),
    );
    invalid_dto(&engine.evaluate(malformed).unwrap_err(), "evaluate");
}

#[wasm_bindgen_test]
fn strict_column_fields_reject_unknown_fields_in_every_map_column_variant() {
    for (kind, ty, column) in [
        (
            "Number",
            dto::ValueType::Number,
            dto::Column::Number(dto::ColumnData {
                values: vec![f64::NAN],
                validity: vec![true],
            }),
        ),
        (
            "String",
            dto::ValueType::String,
            dto::Column::String(dto::ColumnData {
                values: vec!["text".into()],
                validity: vec![true],
            }),
        ),
        (
            "Boolean",
            dto::ValueType::Boolean,
            dto::Column::Boolean(dto::ColumnData {
                values: vec![true],
                validity: vec![true],
            }),
        ),
        (
            "Date",
            dto::ValueType::Date,
            dto::Column::Date(dto::ColumnData {
                values: vec![9_007_199_254_740_993],
                validity: vec![true],
            }),
        ),
        (
            "List",
            dto::ValueType::List(Box::new(dto::ValueType::Unknown)),
            dto::Column::List(dto::ColumnData {
                values: vec![vec![None, Some(dto::Value::Date(i64::MAX))]],
                validity: vec![true],
            }),
        ),
        (
            "Union",
            dto::ValueType::Unknown,
            dto::Column::Union(dto::ColumnData {
                values: vec![dto::Value::List(vec![None])],
                validity: vec![true],
            }),
        ),
    ] {
        let engine = session(vec![
            input("first", dto::ValueType::Number),
            input("second", ty),
            formula("first_out", "prop(\"first\")"),
            formula("second_out", "prop(\"second\")"),
        ]);
        let mut request = empty_input(&["first_out", "second_out"]);
        request.columns.insert(
            "first".into(),
            dto::Column::Number(dto::ColumnData {
                values: vec![1.0],
                validity: vec![true],
            }),
        );
        request.columns.insert("second".into(), column);
        assert!(engine.evaluate(value(&request)).is_ok(), "{kind}");
        let malformed = value(&request);
        let columns = Map::new();
        let serialized: Map = field(&malformed, "columns").dyn_into().unwrap();
        // Keep the malformed column after a valid entry so all Map values
        // must be checked, including the Date custom deserialization path.
        columns.set(
            &JsValue::from_str("first"),
            &serialized.get(&JsValue::from_str("first")),
        );
        let second = serialized.get(&JsValue::from_str("second"));
        set(&field(&second, kind), "extra", &JsValue::NULL);
        columns.set(&JsValue::from_str("second"), &second);
        set(&malformed, "columns", &columns.into());
        invalid_dto(&engine.evaluate(malformed).unwrap_err(), "evaluate");
    }
}

#[wasm_bindgen_test]
fn strict_completion_and_edit_fields_reject_without_reading_unknown_values_or_updating() {
    let mut engine = session(vec![]);
    let handle = engine
        .create_draft(value(&definition("candidate", "1")))
        .unwrap();
    let malformed = value(&dto::CompletionConfig { preferred_limit: 5 });
    set(&malformed, "extra", &JsValue::NULL);
    invalid_dto(
        &engine
            .draft_help(handle, JsValue::from_f64(1.0), malformed)
            .unwrap_err(),
        "draft_help",
    );
    assert!(
        engine
            .draft_help(
                handle,
                JsValue::from_f64(1.0),
                value(&dto::CompletionConfig { preferred_limit: 5 }),
            )
            .is_ok()
    );

    let update = dto::ExpressionUpdate::Edits {
        edit: dto::FormulaEdit {
            base_version: 0,
            edits: vec![dto::TextEdit {
                range: dto::Span { start: 0, end: 1 },
                new_text: "2".into(),
            }],
        },
        cursor: 1,
    };
    let malformed = value(&update);
    let calls = Rc::new(Cell::new(0));
    let callback_calls = Rc::clone(&calls);
    let getter = Closure::<dyn FnMut() -> JsValue>::new(move || {
        callback_calls.set(callback_calls.get() + 1);
        JsValue::from_f64(1.0)
    });
    let descriptor = js_sys::Object::new();
    set(&descriptor, "get", getter.as_ref());
    set(&descriptor, "enumerable", &JsValue::TRUE);
    js_sys::Object::define_property(
        &field(&field(&malformed, "Edits"), "edit")
            .dyn_into::<js_sys::Object>()
            .unwrap(),
        &JsValue::from_str("extra"),
        &descriptor,
    );
    invalid_dto(
        &engine
            .draft_update_expression(handle, malformed)
            .unwrap_err(),
        "draft_update_expression",
    );
    assert_eq!(calls.get(), 0, "unknown field getter must not be called");
    let state = engine.draft_state(handle).unwrap();
    assert_eq!(u64::try_from(field(&state, "version")).unwrap(), 0);
    assert_eq!(
        field(&field(&state, "definition"), "expression"),
        JsValue::from_str("1")
    );
    let updated = engine
        .draft_update_expression(handle, value(&update))
        .unwrap();
    assert_eq!(
        u64::try_from(field(&field(&updated, "state"), "version")).unwrap(),
        1
    );
}

#[wasm_bindgen_test]
fn lossless_columns_maps_values_and_native_evaluation_parity() {
    let schema = dto::FormulaSchema {
        properties: vec![
            input("__proto__", dto::ValueType::Number),
            input("dates", dto::ValueType::Date),
            input(
                "lists",
                dto::ValueType::List(Box::new(dto::ValueType::Unknown)),
            ),
            input("dynamic", dto::ValueType::Unknown),
            formula("constructor", "prop(\"__proto__\")"),
            formula("dateOut", "prop(\"dates\")"),
            formula("listOut", "prop(\"lists\")"),
            formula("dynamicOut", "prop(\"dynamic\")"),
            formula("clock", "now()"),
            formula("toString", "1 +"),
        ],
    };
    let engine = FormulaEngineSession::new(value(&schema)).unwrap();
    let request = dto::EvaluateInput {
        row_ids: (0..4).map(|n| format!("row-{n}")).collect(),
        columns: HashMap::from([
            (
                "__proto__".into(),
                dto::Column::Number(dto::ColumnData {
                    values: vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0],
                    validity: vec![true; 4],
                }),
            ),
            (
                "dates".into(),
                dto::Column::Date(dto::ColumnData {
                    values: vec![i64::MIN, i64::MAX, 9_007_199_254_740_993, 0],
                    validity: vec![true; 4],
                }),
            ),
            (
                "lists".into(),
                dto::Column::List(dto::ColumnData {
                    values: vec![
                        vec![],
                        vec![None],
                        vec![Some(dto::Value::List(vec![
                            None,
                            Some(dto::Value::Date(i64::MAX)),
                        ]))],
                        vec![],
                    ],
                    validity: vec![true, true, true, false],
                }),
            ),
            (
                "dynamic".into(),
                dto::Column::Union(dto::ColumnData {
                    values: vec![
                        dto::Value::Date(i64::MIN),
                        dto::Value::Number(-0.0),
                        dto::Value::List(vec![None]),
                        dto::Value::Boolean(true),
                    ],
                    validity: vec![true; 4],
                }),
            ),
        ]),
        runtime: dto::RuntimeContext {
            now: 1_700_000_123_456,
            time_zone: "+08:00".into(),
        },
        formula_ids: [
            "constructor",
            "dateOut",
            "listOut",
            "dynamicOut",
            "clock",
            "toString",
        ]
        .map(String::from)
        .into(),
    };
    let expected = native::FormulaEngine::new(schema.into())
        .unwrap()
        .evaluate(&request.clone().into())
        .unwrap();
    let actual = engine.evaluate(value(&request)).unwrap();
    let formulas: Map = field(&actual, "formulas").dyn_into().unwrap();
    assert_eq!(formulas.size(), 6);
    assert_eq!(
        field(&formulas.get(&JsValue::from_str("toString")), "Err"),
        JsValue::from_str("NotReady")
    );
    let number_column = field(&field(&output(&actual, "constructor"), "column"), "Number");
    let numbers = array(&field(&number_column, "values"));
    assert!(numbers.get(0).as_f64().unwrap().is_nan());
    assert_eq!(numbers.get(1).as_f64(), Some(f64::INFINITY));
    assert_eq!(numbers.get(2).as_f64(), Some(f64::NEG_INFINITY));
    assert_eq!(
        numbers.get(3).as_f64().unwrap().to_bits(),
        (-0.0f64).to_bits()
    );
    let dates = array(&field(
        &field(&field(&output(&actual, "dateOut"), "column"), "Date"),
        "values",
    ));
    for (index, expected) in [i64::MIN, i64::MAX, 9_007_199_254_740_993, 0]
        .into_iter()
        .enumerate()
    {
        assert!(dates.get(index as u32).is_bigint());
        assert_eq!(i64::try_from(dates.get(index as u32)).unwrap(), expected);
    }
    let lists = field(&field(&output(&actual, "listOut"), "column"), "List");
    let items = array(&field(&lists, "values"));
    assert_eq!(array(&items.get(0)).length(), 0);
    assert!(array(&items.get(1)).get(0).is_null());
    let nested = field(&array(&items.get(2)).get(0), "List");
    assert!(array(&nested).get(0).is_null());
    assert_eq!(
        i64::try_from(field(&array(&nested).get(1), "Date")).unwrap(),
        i64::MAX
    );
    assert_eq!(array(&field(&lists, "validity")).get(3), JsValue::FALSE);
    for id in ["constructor", "dateOut", "listOut", "dynamicOut", "clock"] {
        let expected = expected.formulas[&native::PropertyId::from(id)]
            .as_ref()
            .unwrap();
        let actual_output = output(&actual, id);
        let expected_output: dto::FormulaOutput = expected.clone().into();
        assert_eq!(
            js_sys::JSON::stringify(&field(&actual_output, "output_type")).unwrap(),
            js_sys::JSON::stringify(&value(&expected_output.output_type)).unwrap()
        );
        assert_eq!(array(&field(&actual_output, "errors")).length(), 0);
        let actual_validity = field(
            &field(&actual_output, "column"),
            match &expected.column {
                native::Column::Number(_) => "Number",
                native::Column::Date(_) => "Date",
                native::Column::List(_) => "List",
                native::Column::Union(_) => "Union",
                _ => unreachable!(),
            },
        );
        assert_eq!(array(&field(&actual_validity, "validity")).length(), 4);
    }
    let clock = array(&field(
        &field(&field(&output(&actual, "clock"), "column"), "Date"),
        "values",
    ));
    assert_eq!(i64::try_from(clock.get(0)).unwrap(), request.runtime.now);
}

#[wasm_bindgen_test]
fn native_input_errors_and_transport_validation_are_structured() {
    let engine = session(vec![formula("ok", "1"), formula("bad", "1 +")]);
    let mut request = empty_input(&["ok"]);
    request.runtime.now = i64::MAX;
    let error = engine.evaluate(value(&request)).unwrap_err();
    error_code(&error, "EVALUATE_INPUT");
    let invalid_now = field(&field(&field(&error, "payload"), "error"), "InvalidNow");
    assert_eq!(i64::try_from(field(&invalid_now, "now")).unwrap(), i64::MAX);
    request.runtime.now = 1_700_000_123_456;
    let malformed = value(&request);
    set(
        &field(&malformed, "runtime"),
        "now",
        &JsValue::from_f64(request.runtime.now as f64),
    );
    error_code(&engine.evaluate(malformed).unwrap_err(), "INVALID_DTO");
    let malformed = value(&request);
    set(&malformed, "columns", &js_sys::Object::new().into());
    error_code(&engine.evaluate(malformed).unwrap_err(), "INVALID_DTO");
    request.row_ids = vec!["same".into(), "same".into()];
    let error = engine.evaluate(value(&request)).unwrap_err();
    error_code(&error, "EVALUATE_INPUT");
    assert_eq!(
        field(
            &field(&field(&field(&error, "payload"), "error"), "DuplicateRowId"),
            "id"
        ),
        JsValue::from_str("same")
    );

    let engine = session(vec![
        input("date", dto::ValueType::Date),
        formula("ok", "prop(\"date\")"),
    ]);
    let mut request = empty_input(&["ok"]);
    request.columns.insert(
        "date".into(),
        dto::Column::Date(dto::ColumnData {
            values: vec![0],
            validity: vec![],
        }),
    );
    let error = engine.evaluate(value(&request)).unwrap_err();
    error_code(&error, "EVALUATE_INPUT");
    assert_eq!(
        field(
            &field(
                &field(&field(&error, "payload"), "error"),
                "InvalidColumnLength"
            ),
            "validity_len"
        )
        .as_f64(),
        Some(0.0)
    );
    request.columns.insert(
        "date".into(),
        dto::Column::Date(dto::ColumnData {
            values: vec![0],
            validity: vec![true],
        }),
    );
    let malformed = value(&request);
    let columns: Map = field(&malformed, "columns").dyn_into().unwrap();
    let dates = field(&columns.get(&JsValue::from_str("date")), "Date");
    set(
        &dates,
        "values",
        &Array::of1(&JsValue::from_f64(0.0)).into(),
    );
    error_code(&engine.evaluate(malformed).unwrap_err(), "INVALID_DTO");
}

#[wasm_bindgen_test]
fn draft_snapshots_ids_utf16_edits_and_versions_are_preserved() {
    let mut engine = session(vec![]);
    let source = "// 中文🙂\n[1,2,]\n";
    let first = engine
        .create_draft(value(&definition("first", source)))
        .unwrap();
    let second = engine
        .create_draft(value(&definition("second", source)))
        .unwrap();
    let snapshot = engine.draft_state(first).unwrap();
    assert_eq!(u64::try_from(field(&snapshot, "version")).unwrap(), 0);
    let reread = engine.draft_state(first).unwrap();
    let diagnostic = array(&field(&snapshot, "diagnostics")).get(0);
    let id = field(&diagnostic, "id").as_string().unwrap();
    assert_eq!(
        field(&array(&field(&reread, "diagnostics")).get(0), "id"),
        JsValue::from_str(&id)
    );
    assert_eq!(
        array(&engine.draft_quick_fixes(second, id.clone()).unwrap()).length(),
        0
    );
    let fixes = array(&engine.draft_quick_fixes(first, id.clone()).unwrap());
    assert!(fixes.length() > 0);
    let edit = field(&fixes.get(0), "edit");
    assert_eq!(u64::try_from(field(&edit, "base_version")).unwrap(), 0);
    let tokens = array(&field(&snapshot, "tokens"));
    assert_eq!(
        field(&tokens.get(0), "text"),
        JsValue::from_str("// 中文🙂")
    );
    assert_eq!(
        field(&field(&tokens.get(0), "span"), "end").as_f64(),
        Some(7.0)
    );
    assert_eq!(
        field(&tokens.get(tokens.length() - 1), "kind"),
        JsValue::from_str("Eof")
    );
    assert_eq!(
        field(&field(&tokens.get(tokens.length() - 1), "span"), "end").as_f64(),
        Some(source.encode_utf16().count() as f64)
    );
    let update = js_sys::Object::new();
    let edits = js_sys::Object::new();
    set(&edits, "edit", &edit);
    set(
        &edits,
        "cursor",
        &JsValue::from_f64(source.encode_utf16().count() as f64),
    );
    set(&update, "Edits", &edits.into());
    let result = engine
        .draft_update_expression(first, update.into())
        .unwrap();
    assert_eq!(
        u64::try_from(field(&field(&result, "state"), "version")).unwrap(),
        1
    );
    assert_eq!(
        field(&field(&snapshot, "definition"), "expression"),
        JsValue::from_str(source)
    );
    assert_eq!(
        array(&engine.draft_quick_fixes(first, id).unwrap()).length(),
        0
    );
    let stale = dto::ExpressionUpdate::Edits {
        edit: serde_wasm_bindgen::from_value(edit).unwrap(),
        cursor: 0,
    };
    let error = engine
        .draft_update_expression(first, value(&stale))
        .unwrap_err();
    error_code(&error, "UPDATE_EXPRESSION");
    assert_eq!(
        field(&field(&error, "payload"), "error"),
        JsValue::from_str("VersionMismatch")
    );

    engine
        .draft_update_expression(
            first,
            value(&dto::ExpressionUpdate::Replace("\"中🙂文\"".into())),
        )
        .unwrap();
    let state = engine.draft_state(first).unwrap();
    let version = u64::try_from(field(&state, "version")).unwrap();
    let update = dto::ExpressionUpdate::Edits {
        edit: dto::FormulaEdit {
            base_version: version,
            edits: vec![dto::TextEdit {
                range: dto::Span { start: 3, end: 4 },
                new_text: "X".into(),
            }],
        },
        cursor: 6,
    };
    let result = engine
        .draft_update_expression(first, value(&update))
        .unwrap();
    assert_eq!(
        field(&field(&field(&result, "state"), "definition"), "expression"),
        JsValue::from_str("\"中X文\"")
    );
    assert_eq!(field(&result, "cursor").as_f64(), Some(5.0));
    let version = u64::try_from(field(&field(&result, "state"), "version")).unwrap();
    for error_case in [
        dto::ExpressionUpdate::Edits {
            edit: dto::FormulaEdit {
                base_version: version,
                edits: vec![],
            },
            cursor: 100,
        },
        dto::ExpressionUpdate::Edits {
            edit: dto::FormulaEdit {
                base_version: version,
                edits: vec![dto::TextEdit {
                    range: dto::Span { start: 4, end: 2 },
                    new_text: "".into(),
                }],
            },
            cursor: 0,
        },
        dto::ExpressionUpdate::Edits {
            edit: dto::FormulaEdit {
                base_version: version,
                edits: vec![
                    dto::TextEdit {
                        range: dto::Span { start: 0, end: 3 },
                        new_text: "".into(),
                    },
                    dto::TextEdit {
                        range: dto::Span { start: 2, end: 4 },
                        new_text: "".into(),
                    },
                ],
            },
            cursor: 0,
        },
    ] {
        error_code(
            &engine
                .draft_update_expression(first, value(&error_case))
                .unwrap_err(),
            "UPDATE_EXPRESSION",
        );
        assert_eq!(
            u64::try_from(field(&engine.draft_state(first).unwrap(), "version")).unwrap(),
            version
        );
    }
    let bad_version = value(&dto::ExpressionUpdate::Edits {
        edit: dto::FormulaEdit {
            base_version: version,
            edits: vec![],
        },
        cursor: 0,
    });
    set(
        &field(&field(&bad_version, "Edits"), "edit"),
        "base_version",
        &JsValue::from_f64(version as f64),
    );
    error_code(
        &engine
            .draft_update_expression(first, bad_version)
            .unwrap_err(),
        "INVALID_DTO",
    );
}

#[wasm_bindgen_test]
fn completion_formatting_nulls_and_controlled_lifecycle() {
    let mut engine = session(vec![
        input("amount", dto::ValueType::Number),
        formula("saved", "1"),
    ]);
    assert!(engine.get_property("missing".into()).unwrap().is_null());
    assert!(engine.remove("missing".into()).unwrap().is_null());
    let draft = engine
        .create_draft(value(&definition("candidate", "amo")))
        .unwrap();
    let help = engine
        .draft_help(
            draft,
            JsValue::from_f64(3.0),
            value(&dto::CompletionConfig { preferred_limit: 5 }),
        )
        .unwrap();
    assert!(field(&help, "signature_help").is_null());
    let items = array(&field(&field(&help, "completion"), "items"));
    let amount = items
        .iter()
        .find(|item| field(item, "label") == JsValue::from_str("amount"))
        .unwrap();
    assert!(field(&amount, "disabled_reason").is_null());
    let candidate = items
        .iter()
        .find(|item| field(item, "label") == JsValue::from_str("candidate"))
        .unwrap();
    assert!(field(&candidate, "primary_edit").is_null());
    assert!(field(&candidate, "cursor").is_null());
    for cursor in [
        JsValue::from_f64(-1.0),
        JsValue::from_f64(0.5),
        JsValue::from_f64(f64::NAN),
        JsValue::from_f64(4_294_967_296.0),
    ] {
        error_code(
            &engine
                .draft_help(
                    draft,
                    cursor,
                    value(&dto::CompletionConfig { preferred_limit: 5 }),
                )
                .unwrap_err(),
            "INVALID_DTO",
        );
    }
    assert!(
        engine
            .draft_help(
                draft,
                JsValue::from_f64(100.0),
                value(&dto::CompletionConfig { preferred_limit: 0 })
            )
            .is_ok()
    );
    engine
        .draft_update_expression(draft, value(&dto::ExpressionUpdate::Replace("1 +".into())))
        .unwrap();
    error_code(
        &engine.draft_format_edits(draft).unwrap_err(),
        "FORMAT_ERROR",
    );
    engine
        .draft_update_expression(
            draft,
            value(&dto::ExpressionUpdate::Replace("/* 中🙂 */ 1+2".into())),
        )
        .unwrap();
    let edit = engine.draft_format_edits(draft).unwrap();
    let text_edit = array(&field(&edit, "edits")).get(0);
    assert_eq!(
        field(&field(&text_edit, "range"), "end").as_f64(),
        Some("/* 中🙂 */ 1+2".encode_utf16().count() as f64)
    );
    assert!(
        field(&text_edit, "new_text")
            .as_string()
            .unwrap()
            .ends_with('\n')
    );
    let error = engine.upsert(value(&formula("saved", "2"))).unwrap_err();
    error_code(&error, "ACTIVE_DRAFTS");
    assert_eq!(
        field(&field(&error, "payload"), "count").as_f64(),
        Some(1.0)
    );
    error_code(
        &engine.remove("missing".into()).unwrap_err(),
        "ACTIVE_DRAFTS",
    );
    let completed_value = engine.draft_into_definition(draft).unwrap();
    error_code(
        &engine.draft_into_definition(draft).unwrap_err(),
        "DRAFT_CLOSED",
    );
    engine.draft_close(draft);
    engine.draft_close(draft);
    let completed: dto::FormulaDefinition =
        serde_wasm_bindgen::from_value(completed_value).unwrap();
    engine
        .upsert(value(&dto::PropertyDefinition::Formula(completed)))
        .unwrap();
    let another = engine
        .create_draft(value(&definition("another", "1")))
        .unwrap();
    engine.close();
    engine.close();
    engine.draft_close(another);
    let error = engine.get_state().unwrap_err();
    error_code(&error, "ENGINE_CLOSED");
    assert!(field(&error, "payload").is_null());
    error_code(&engine.draft_state(another).unwrap_err(), "ENGINE_CLOSED");
}

#[wasm_bindgen_test]
fn formula_row_failures_and_schema_errors_preserve_native_payloads() {
    let mut engine = session(vec![
        input("text", dto::ValueType::String),
        input("count", dto::ValueType::Number),
        formula("repeat", "repeat(prop(\"text\"), prop(\"count\"))"),
        formula(
            "dependent",
            "join([prop(\"repeat\"), prop(\"repeat\")], \"\")",
        ),
    ]);
    let request = dto::EvaluateInput {
        row_ids: vec!["failure".into(), "success".into()],
        columns: HashMap::from([
            (
                "text".into(),
                dto::Column::String(dto::ColumnData {
                    values: vec!["ha".into(), "go".into()],
                    validity: vec![true; 2],
                }),
            ),
            (
                "count".into(),
                dto::Column::Number(dto::ColumnData {
                    values: vec![f64::INFINITY, 2.0],
                    validity: vec![true; 2],
                }),
            ),
        ]),
        runtime: dto::RuntimeContext {
            now: 1_700_000_123_456,
            time_zone: "+08:00".into(),
        },
        formula_ids: vec!["repeat".into(), "dependent".into()],
    };
    let result = engine.evaluate(value(&request)).unwrap();
    for (id, text) in [("repeat", "gogo"), ("dependent", "gogogogo")] {
        let out = output(&result, id);
        let column = field(&field(&out, "column"), "String");
        assert_eq!(array(&field(&column, "validity")).get(0), JsValue::FALSE);
        assert_eq!(array(&field(&column, "validity")).get(1), JsValue::TRUE);
        assert_eq!(
            array(&field(&column, "values")).get(1),
            JsValue::from_str(text)
        );
        let errors = array(&field(&out, "errors"));
        assert_eq!(errors.length(), 1);
        assert_eq!(field(&errors.get(0), "row_index").as_f64(), Some(0.0));
        assert_eq!(
            field(&errors.get(0), "origin_formula_id"),
            JsValue::from_str("repeat")
        );
        let invalid = field(&field(&errors.get(0), "error"), "InvalidValue");
        assert_eq!(
            field(&field(&invalid, "actual"), "Number").as_f64(),
            Some(f64::INFINITY)
        );
        assert!(
            !field(&invalid, "constraint")
                .as_string()
                .unwrap()
                .is_empty()
        );
    }
    let error = engine.upsert(value(&formula("", "1"))).unwrap_err();
    error_code(&error, "ENGINE_CHANGE");
    assert_eq!(
        field(&field(&error, "payload"), "error"),
        JsValue::from_str("EmptyId")
    );
    let error = engine
        .create_draft(value(&definition("", "1")))
        .unwrap_err();
    error_code(&error, "CREATE_DRAFT");
    assert_eq!(
        field(&field(&error, "payload"), "error"),
        JsValue::from_str("EmptyId")
    );
    let duplicate = dto::FormulaSchema {
        properties: vec![formula("duplicate", "1"), formula("duplicate", "2")],
    };
    let error = match FormulaEngineSession::new(value(&duplicate)) {
        Err(error) => error,
        Ok(_) => panic!("duplicate IDs must fail"),
    };
    error_code(&error, "ENGINE_INIT");
    assert_eq!(
        field(&field(&field(&error, "payload"), "error"), "DuplicateId"),
        JsValue::from_str("duplicate")
    );
}

#[wasm_bindgen_test]
fn unicode_completion_and_signature_indices_remain_utf16_numbers() {
    let mut engine = session(vec![input("中🙂", dto::ValueType::Number)]);
    let handle = engine
        .create_draft(value(&definition("candidate", "中")))
        .unwrap();
    let help = engine
        .draft_help(
            handle,
            JsValue::from_f64(1.0),
            value(&dto::CompletionConfig { preferred_limit: 5 }),
        )
        .unwrap();
    let items = array(&field(&field(&help, "completion"), "items"));
    let item = items
        .iter()
        .find(|item| field(item, "label") == JsValue::from_str("中🙂"))
        .unwrap();
    let edit = field(&item, "primary_edit");
    let new_text = field(&edit, "new_text").as_string().unwrap();
    assert_eq!(
        field(&item, "cursor").as_f64(),
        Some(new_text.encode_utf16().count() as f64)
    );
    assert_eq!(field(&field(&edit, "range"), "end").as_f64(), Some(1.0));
    let source = "repeat(\"中🙂\", ";
    engine
        .draft_update_expression(
            handle,
            value(&dto::ExpressionUpdate::Replace(source.into())),
        )
        .unwrap();
    let help = engine
        .draft_help(
            handle,
            JsValue::from_f64(source.encode_utf16().count() as f64),
            value(&dto::CompletionConfig { preferred_limit: 5 }),
        )
        .unwrap();
    let signature = field(&help, "signature_help");
    assert_eq!(field(&signature, "active_signature").as_f64(), Some(0.0));
    assert_eq!(field(&signature, "active_parameter").as_f64(), Some(1.0));
    assert!(field(&help, "base_version").is_bigint());
    engine
        .draft_update_expression(handle, value(&dto::ExpressionUpdate::Replace("rep".into())))
        .unwrap();
    let help = engine
        .draft_help(
            handle,
            JsValue::from_f64(3.0),
            value(&dto::CompletionConfig { preferred_limit: 5 }),
        )
        .unwrap();
    let indices = array(&field(&field(&help, "completion"), "preferred_indices"));
    assert!(indices.length() > 0);
    assert!(indices.iter().all(|index| index.as_f64().is_some()));
}
