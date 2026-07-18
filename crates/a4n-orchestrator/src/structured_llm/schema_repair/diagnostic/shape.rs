use serde_json::{Value, json};

pub(crate) fn json_shape_schema_from_example(example: &Value) -> Value {
    match example {
        Value::Object(object) => {
            let properties = object
                .iter()
                .map(|(key, value)| (key.clone(), json_shape_schema_from_example(value)))
                .collect::<serde_json::Map<_, _>>();
            json!({
                "type": "object",
                "properties": properties,
                "required": object.keys().collect::<Vec<_>>(),
                "additionalProperties": false,
            })
        }
        Value::Array(values) => json!({
            "type": "array",
            "items": values
                .first()
                .map(json_shape_schema_from_example)
                .unwrap_or_else(|| json!({})),
        }),
        Value::String(value) => {
            let variants = value
                .split('|')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>();
            if variants.len() > 1 {
                json!({"type": "string", "enum": variants})
            } else {
                json!({"type": "string"})
            }
        }
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            json!({"type": "integer"})
        }
        Value::Number(_) => json!({"type": "number"}),
        Value::Bool(_) => json!({"type": "boolean"}),
        Value::Null => json!({"type": "null"}),
    }
}
