use serde_json::Value;

pub(super) fn extract_character_names_from_value(value: &Value) -> Vec<String> {
    let mut names = Vec::new();
    collect_character_names(value, &mut names);
    names.sort();
    names.dedup();
    names
}

fn collect_character_names(value: &Value, names: &mut Vec<String>) {
    match value {
        Value::String(text) => push_name(names, text),
        Value::Array(items) => {
            for item in items {
                collect_character_names(item, names);
            }
        }
        Value::Object(map) => {
            for key in ["name", "character", "character_name", "人物", "角色"] {
                if let Some(text) = map.get(key).and_then(Value::as_str) {
                    push_name(names, text);
                }
            }
        }
        _ => {}
    }
}

fn push_name(names: &mut Vec<String>, text: &str) {
    let text = text.trim();
    if !text.is_empty() {
        names.push(text.to_string());
    }
}

pub fn truncate_text_chars(text: &str, max_chars: usize) -> String {
    let mut iter = text.trim().chars();
    let truncated = iter.by_ref().take(max_chars).collect::<String>();
    if iter.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub fn compact_json_text(value: &Value, max_chars: usize) -> String {
    let text = match value {
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    };
    truncate_text_chars(&text, max_chars)
}

pub fn is_meaningful_json_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(_) | Value::Number(_) | Value::String(_) => true,
        Value::Array(items) => !items.is_empty() && items.iter().any(is_meaningful_json_value),
        Value::Object(map) => !map.is_empty() && map.values().any(is_meaningful_json_value),
    }
}
