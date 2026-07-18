use serde_json::{Value, json};

use super::super::json_utils::{required_string, required_value};

pub(in crate::actions::character_ai) fn discovered_character_base_attributes(
    character: &Value,
) -> Result<Value, String> {
    let mut explicit = required_value(character, "explicit_attributes")?
        .as_object()
        .ok_or_else(|| "角色发现字段 explicit_attributes 必须是 JSON 对象".to_string())?
        .clone();
    let mut implicit = required_value(character, "implicit_attributes")?
        .as_object()
        .ok_or_else(|| "角色发现字段 implicit_attributes 必须是 JSON 对象".to_string())?
        .clone();
    let social = required_value(character, "social_attributes")?
        .as_object()
        .ok_or_else(|| "角色发现字段 social_attributes 必须是 JSON 对象".to_string())?
        .clone();

    explicit.insert(
        "role".to_string(),
        Value::String(required_string(character, "role")?),
    );
    explicit.insert(
        "aliases".to_string(),
        required_value(character, "aliases")?.clone(),
    );
    explicit.insert(
        "first_seen_chapter".to_string(),
        required_value(character, "first_seen_chapter")?.clone(),
    );
    explicit.insert(
        "latest_seen_chapter".to_string(),
        required_value(character, "latest_seen_chapter")?.clone(),
    );
    explicit.insert(
        "importance".to_string(),
        Value::String(required_string(character, "importance")?),
    );
    explicit.insert(
        "evidence".to_string(),
        Value::String(required_string(character, "evidence")?),
    );
    insert_optional_attribute(
        &mut explicit,
        "appearance",
        required_value(character, "appearance")?,
    );
    insert_optional_attribute(
        &mut explicit,
        "background",
        required_value(character, "background")?,
    );
    insert_optional_attribute(
        &mut implicit,
        "personality",
        required_value(character, "personality")?,
    );

    Ok(json!({
        "explicit_attributes": explicit,
        "implicit_attributes": implicit,
        "social_attributes": social,
    }))
}

fn insert_optional_attribute(
    attributes: &mut serde_json::Map<String, Value>,
    key: &str,
    value: &Value,
) {
    if !value.is_null() && !attributes.contains_key(key) {
        attributes.insert(key.to_string(), value.clone());
    }
}
