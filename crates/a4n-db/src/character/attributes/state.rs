use super::super::CharacterDbError;
use serde_json::Value;

pub(in crate::character) fn category_storage_key(category: &str) -> &'static str {
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
) -> Result<(), CharacterDbError> {
    let storage_key = category_storage_key(category);
    let root = state
        .as_object_mut()
        .ok_or_else(|| CharacterDbError::InvalidState {
            current: "角色当前状态不是 JSON 对象".to_string(),
            operation: "应用角色属性变更".to_string(),
        })?;
    let attributes = root
        .get_mut(storage_key)
        .ok_or_else(|| CharacterDbError::InvalidState {
            current: format!("角色当前状态缺少 {storage_key}"),
            operation: "应用角色属性变更".to_string(),
        })?
        .as_object_mut()
        .ok_or_else(|| CharacterDbError::InvalidState {
            current: format!("角色当前状态 {storage_key} 不是 JSON 对象"),
            operation: "应用角色属性变更".to_string(),
        })?;
    if operation == "delete" {
        attributes.remove(key);
    } else {
        attributes.insert(key.to_string(), new_value);
    }
    Ok(())
}

pub(in crate::character) fn attribute_value_in_state(
    state: &Value,
    category: &str,
    key: &str,
) -> Option<Value> {
    state
        .get(category_storage_key(category))
        .and_then(Value::as_object)
        .and_then(|attributes| attributes.get(key))
        .cloned()
}
