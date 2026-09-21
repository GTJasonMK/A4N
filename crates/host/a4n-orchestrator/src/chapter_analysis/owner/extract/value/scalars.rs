use serde_json::Value;

pub fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .find_map(value_as_string)
}

pub fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            let text = s.trim();
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }
        }
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
