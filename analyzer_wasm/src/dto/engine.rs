//! Engine/Draft DTOs. Unlike the legacy Analyzer serializer, this boundary uses
//! bigint for 64-bit integers, Map for IDs, and explicit null for absent values.

include!("engine.h.rs");

use js_sys::{Array, Map, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

// serde-wasm-bindgen reads struct fields directly by name, so serde's
// deny_unknown_fields attribute does not see extra JS object properties.
// Check just the strict DTO shapes before deserialization; serde and the
// native engine still own transport types and domain validation respectively.
pub(crate) fn validate_fields(value: &JsValue, allowed: &[&str]) -> Result<(), JsValue> {
    if !value.is_object() {
        return Err(JsValue::UNDEFINED);
    }
    for key in Object::keys(value.unchecked_ref::<Object>()).iter() {
        if !key
            .as_string()
            .is_some_and(|key| allowed.contains(&key.as_str()))
        {
            return Err(JsValue::UNDEFINED);
        }
    }
    Ok(())
}

fn field(value: &JsValue, name: &str) -> Result<JsValue, JsValue> {
    Reflect::get(value, &JsValue::from_str(name))
}

pub(crate) fn validate_schema_fields(value: &JsValue) -> Result<(), JsValue> {
    validate_fields(value, &["properties"])?;
    let properties = field(value, "properties")?.dyn_into::<Array>()?;
    for property in properties.iter() {
        validate_property_fields(&property)?;
    }
    Ok(())
}

pub(crate) fn validate_property_fields(value: &JsValue) -> Result<(), JsValue> {
    let formula = field(value, "Formula")?;
    if !formula.is_undefined() {
        validate_formula_fields(&formula)?;
    }
    Ok(())
}

pub(crate) fn validate_formula_fields(value: &JsValue) -> Result<(), JsValue> {
    validate_fields(value, &["id", "expression"])
}

pub(crate) fn validate_evaluate_fields(value: &JsValue) -> Result<(), JsValue> {
    validate_fields(value, &["row_ids", "columns", "runtime", "formula_ids"])?;
    validate_fields(&field(value, "runtime")?, &["now", "time_zone"])?;
    let columns = field(value, "columns")?.dyn_into::<Map>()?;
    for column in columns.values() {
        let column = column?;
        if !column.is_object() {
            return Err(JsValue::UNDEFINED);
        }
        for tag in Object::keys(column.unchecked_ref::<Object>()).iter() {
            if let Some(tag) = tag.as_string()
                && matches!(
                    tag.as_str(),
                    "Number" | "String" | "Boolean" | "Date" | "DateValue" | "List" | "Union"
                )
            {
                let data = field(&column, &tag)?;
                validate_fields(&data, &["values", "validity"])?;
                if matches!(tag.as_str(), "DateValue" | "List" | "Union") {
                    let values = field(&data, "values")?.dyn_into::<Array>()?;
                    for value in values.iter() {
                        match tag.as_str() {
                            "DateValue" => {
                                validate_fields(&value, &["start", "end", "include_time"])?
                            }
                            "List" => validate_list_value_fields(&value)?,
                            _ => validate_value_fields(&value)?,
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_list_value_fields(value: &JsValue) -> Result<(), JsValue> {
    for value in value.clone().dyn_into::<Array>()?.iter() {
        validate_value_fields(&value)?;
    }
    Ok(())
}

fn validate_value_fields(value: &JsValue) -> Result<(), JsValue> {
    if value.is_object() {
        for tag in Object::keys(value.unchecked_ref::<Object>()).iter() {
            match tag.as_string().as_deref() {
                Some("DateValue") => validate_fields(
                    &field(value, "DateValue")?,
                    &["start", "end", "include_time"],
                )?,
                Some("List") => validate_list_value_fields(&field(value, "List")?)?,
                _ => {}
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_completion_fields(value: &JsValue) -> Result<(), JsValue> {
    validate_fields(value, &["preferred_limit"])
}

pub(crate) fn validate_update_fields(value: &JsValue) -> Result<(), JsValue> {
    let edits = field(value, "Edits")?;
    if !edits.is_undefined() {
        validate_fields(&field(&edits, "edit")?, &["base_version", "edits"])?;
    }
    Ok(())
}

fn deserialize_i64_bigint<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let value: wasm_bindgen::JsValue = serde_wasm_bindgen::preserve::deserialize(d)?;
    if !value.is_bigint() {
        return Err(serde::de::Error::custom("expected bigint"));
    }
    i64::try_from(value).map_err(|_| serde::de::Error::custom("bigint exceeds i64"))
}

fn deserialize_u64_bigint<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let value: wasm_bindgen::JsValue = serde_wasm_bindgen::preserve::deserialize(d)?;
    if !value.is_bigint() {
        return Err(serde::de::Error::custom("expected bigint"));
    }
    u64::try_from(value).map_err(|_| serde::de::Error::custom("bigint exceeds u64"))
}

fn deserialize_optional_i64_bigint<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<i64>, D::Error> {
    let value: wasm_bindgen::JsValue = serde_wasm_bindgen::preserve::deserialize(d)?;
    if value.is_null() {
        return Ok(None);
    }
    if !value.is_bigint() {
        return Err(serde::de::Error::custom("expected bigint or null"));
    }
    i64::try_from(value)
        .map(Some)
        .map_err(|_| serde::de::Error::custom("bigint exceeds i64"))
}

fn deserialize_date_column<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<ColumnData<i64>, D::Error> {
    #[derive(Deserialize)]
    struct DateValue(#[serde(deserialize_with = "deserialize_i64_bigint")] i64);
    let column = ColumnData::<DateValue>::deserialize(d)?;
    Ok(ColumnData {
        values: column.values.into_iter().map(|value| value.0).collect(),
        validity: column.validity,
    })
}

fn deserialize_columns<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<HashMap<PropertyId, Column>, D::Error> {
    use wasm_bindgen::JsCast;
    let value: wasm_bindgen::JsValue = serde_wasm_bindgen::preserve::deserialize(d)?;
    if !value.is_instance_of::<js_sys::Map>() {
        return Err(serde::de::Error::custom("expected Map"));
    }
    serde_wasm_bindgen::from_value(value).map_err(serde::de::Error::custom)
}
