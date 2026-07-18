use serde_json::{Value, json};

pub fn json_value_shape_summary(value: &Value) -> Value {
    match value {
        Value::Object(object) => json!({
            "kind": "object",
            "keys": object.keys().cloned().collect::<Vec<_>>(),
            "array_lengths": object
                .iter()
                .filter_map(|(key, value)| {
                    value.as_array().map(|items| json!({
                        "key": key,
                        "len": items.len(),
                    }))
                })
                .collect::<Vec<_>>(),
        }),
        Value::Array(items) => json!({
            "kind": "array",
            "len": items.len(),
        }),
        Value::String(text) => json!({
            "kind": "string",
            "chars": text.chars().count(),
        }),
        Value::Bool(_) => json!({ "kind": "bool" }),
        Value::Number(_) => json!({ "kind": "number" }),
        Value::Null => json!({ "kind": "null" }),
    }
}
