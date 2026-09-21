use a4n_domain::json_contract::is_empty_json_value;
use serde_json::{Map, Value};

pub(super) fn merge_missing_seed_attributes(target: &mut Value, seed: &Value) -> bool {
    let Some(target_root) = target.as_object_mut() else {
        return false;
    };
    let Some(seed_root) = seed.as_object() else {
        return false;
    };
    let mut changed = false;
    for category in [
        "explicit_attributes",
        "implicit_attributes",
        "social_attributes",
    ] {
        let Some(seed_attributes) = seed_root.get(category).and_then(Value::as_object) else {
            continue;
        };
        let target_attributes = target_root
            .entry(category.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !target_attributes.is_object() {
            continue;
        }
        let Some(target_attributes) = target_attributes.as_object_mut() else {
            continue;
        };
        for (key, value) in seed_attributes {
            if is_empty_json_value(value)
                || !is_empty_json_value(target_attributes.get(key).unwrap_or(&Value::Null))
            {
                continue;
            }
            target_attributes.insert(key.clone(), value.clone());
            changed = true;
        }
    }
    changed
}
