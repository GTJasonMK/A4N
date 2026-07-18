use serde_json::Value;

mod array;
mod map;
#[cfg(test)]
mod tests;
use array::value_to_string_list;
use map::map_value_to_string_list;

fn value_to_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_owned())
        }
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

pub fn collect_json_strings(value: Option<&Value>, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }

    match value {
        Some(Value::Array(_)) => value_to_string_list(value, limit),
        Some(Value::Object(_)) => map_value_to_string_list(value, limit),
        Some(Value::String(_)) => value_to_string(value).into_iter().collect(),
        _ => Vec::new(),
    }
}

pub fn stringify_json_value(value: &Value, max_chars: usize) -> String {
    let text = if let Some(text) = value.as_str() {
        text.to_string()
    } else {
        value.to_string()
    };
    let trimmed = text.trim().to_string();
    if trimmed.chars().count() > max_chars {
        let mut truncated = trimmed.chars().take(max_chars).collect::<String>();
        truncated.push('…');
        truncated
    } else {
        trimmed
    }
}
