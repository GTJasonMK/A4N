use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterProfileCurrentStateFields {
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub current_state: Value,
}

fn optional_profile_text_field(attributes: &Value, category: &str, key: &str) -> Option<String> {
    attributes
        .get(category_storage_key(category))
        .and_then(Value::as_object)
        .and_then(|values| values.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

pub fn profile_current_state_fields(current_state: Value) -> CharacterProfileCurrentStateFields {
    CharacterProfileCurrentStateFields {
        gender: optional_profile_text_field(&current_state, "explicit", "gender"),
        age: optional_profile_text_field(&current_state, "explicit", "age"),
        appearance: optional_profile_text_field(&current_state, "explicit", "appearance"),
        background: optional_profile_text_field(&current_state, "explicit", "background"),
        personality: optional_profile_text_field(&current_state, "implicit", "personality"),
        current_state,
    }
}

pub fn profile_reset_base(mut base: Value, text_discovered: bool) -> Result<Value, String> {
    if text_discovered {
        for category in ["explicit", "implicit", "social"] {
            let key = category_storage_key(category);
            base.get_mut(key)
                .and_then(Value::as_object_mut)
                .ok_or_else(|| format!("角色基础属性缺少对象字段 {key}"))?
                .clear();
        }
    }
    Ok(base)
}

pub fn initial_snapshot_data() -> Value {
    json!({
        "synced_chapter": 0,
        "state_summary": "初始空状态",
        "changes_in_chapter": 0,
        "behaviors_in_chapter": 0,
        "attributes": {
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
        },
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {},
    })
}

pub fn category_storage_key(category: &str) -> &'static str {
    match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        _ => "attributes",
    }
}

pub fn apply_attribute_to_state(
    state: &mut Value,
    category: &str,
    key: &str,
    operation: &str,
    new_value: Value,
) -> Result<(), String> {
    let storage_key = category_storage_key(category);
    let root = state
        .as_object_mut()
        .ok_or_else(|| "角色当前状态不是 JSON 对象".to_string())?;
    let attributes = root
        .get_mut(storage_key)
        .ok_or_else(|| format!("角色当前状态缺少 {storage_key}"))?
        .as_object_mut()
        .ok_or_else(|| format!("角色当前状态 {storage_key} 不是 JSON 对象"))?;
    if operation == "delete" {
        attributes.remove(key);
    } else {
        attributes.insert(key.to_string(), new_value);
    }
    Ok(())
}
