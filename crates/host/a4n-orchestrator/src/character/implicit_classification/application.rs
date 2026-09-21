use a4n_domain::character::apply_attribute_to_state;
use serde_json::{Value, json};

use crate::character::{
    category_storage_key, required_array, required_i32, required_string, required_value,
};

pub(super) fn apply_implicit_classification_to_behavior(
    behavior: &mut Value,
    parsed: &Value,
    current_state: &mut Value,
    sequence: usize,
) -> Result<Vec<Value>, String> {
    let chapter = required_i32(behavior, "chapter_number")?;
    let behavior_description = required_string(behavior, "behavior_description")?;
    let evidence = required_string(behavior, "original_text")?;
    let reasoning = required_string(parsed, "reasoning")?;
    let classifications = required_value(parsed, "classifications")?.clone();
    set_behavior_classification_results(behavior, classifications)?;

    let suggestions = required_array(parsed, "suggested_new_attributes")?;
    let mut applied = Vec::new();
    for item in suggestions {
        let key = required_string(item, "key")?;
        if state_has_implicit_attribute(current_state, &key) {
            continue;
        }
        let value = required_value(item, "value")?.clone();
        apply_attribute_to_state(current_state, "implicit", &key, "add", value.clone())
            .map_err(|err| err.to_string())?;
        applied.push(json!({
            "chapter_number": chapter,
            "key": key,
            "value": value,
            "evidence": evidence,
            "reasoning": reasoning,
            "behavior_description": behavior_description,
            "sequence": sequence,
        }));
    }
    Ok(applied)
}

pub(super) fn set_behavior_classification_results(
    behavior: &mut Value,
    classification_results: Value,
) -> Result<(), String> {
    let object = behavior
        .as_object_mut()
        .ok_or_else(|| "behavior 必须是 JSON 对象".to_string())?;
    object.insert("classification_results".to_string(), classification_results);
    Ok(())
}

fn state_has_implicit_attribute(state: &Value, key: &str) -> bool {
    state
        .get(category_storage_key("implicit"))
        .and_then(Value::as_object)
        .is_some_and(|attributes| attributes.contains_key(key))
}
