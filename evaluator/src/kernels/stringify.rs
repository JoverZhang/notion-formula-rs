use crate::core::types::Value;

pub(crate) fn stringify_list(list: &[Value]) -> String {
    let mut out = String::from("[");
    for (idx, item) in list.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(stringify_value(item).as_str());
    }
    out.push(']');
    out
}

pub(crate) fn stringify_value(value: &Value) -> String {
    match value {
        Value::Number(value) => value.to_string(),
        Value::Text(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Date(value) => value.to_string(),
        Value::List(values) => stringify_list(values),
    }
}
