use super::super::CharacterDbError;
use a4n_domain::character::apply_attribute_to_state as apply_domain_attribute_to_state;
use serde_json::Value;

pub(in crate::character) use a4n_domain::character::category_storage_key;

pub(in crate::character) fn apply_attribute_to_state(
    state: &mut Value,
    category: &str,
    key: &str,
    operation: &str,
    new_value: Value,
) -> Result<(), CharacterDbError> {
    apply_domain_attribute_to_state(state, category, key, operation, new_value).map_err(|current| {
        CharacterDbError::InvalidState {
            current,
            operation: "应用角色属性变更".to_string(),
        }
    })
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
