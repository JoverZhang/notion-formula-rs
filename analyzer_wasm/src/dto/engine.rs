//! Engine/Draft DTOs. Unlike the legacy Analyzer serializer, this boundary uses
//! bigint for 64-bit integers, Map for IDs, and explicit null for absent values.

include!("engine.h.rs");

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
