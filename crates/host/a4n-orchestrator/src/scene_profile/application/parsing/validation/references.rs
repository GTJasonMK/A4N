use a4n_domain::json_contract::{ensure_object, required_array, required_string};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashSet};

pub(in crate::scene_profile::application::parsing) fn validate_profile_refs(
    value: &Value,
    key: &str,
    path: &str,
    allowed_ids: &BTreeSet<i64>,
    id_field: &str,
) -> Result<Value, String> {
    let refs = required_array(value, key, path)?;
    for (index, item) in refs.iter().enumerate() {
        let item_path = format!("{path}.{key}[{index}]");
        ensure_object(item, &item_path)?;
        let id = item
            .get(id_field)
            .and_then(Value::as_i64)
            .ok_or_else(|| format!("{item_path}.{id_field} 必须是已有档案的整数 ID"))?;
        validate_id_in_set(id, allowed_ids, &format!("{item_path}.{id_field}"))?;
        required_string(item, "role", &item_path)?;
    }
    Ok(Value::Array(refs.clone()))
}

pub(in crate::scene_profile::application::parsing) fn id_array_value(
    value: &Value,
    key: &str,
    path: &str,
    allowed_ids: &BTreeSet<i64>,
    label: &str,
) -> Result<Value, String> {
    let values = required_array(value, key, path)?;
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for (index, item) in values.iter().enumerate() {
        let id = item
            .as_i64()
            .ok_or_else(|| format!("{path}.{key}[{index}] 必须是 {label} 整数 ID"))?;
        validate_id_in_set(id, allowed_ids, &format!("{path}.{key}[{index}]"))?;
        if seen.insert(id) {
            ids.push(id);
        }
    }
    Ok(json!(ids))
}

pub(in crate::scene_profile::application::parsing) fn validate_id_in_set(
    id: i64,
    allowed_ids: &BTreeSet<i64>,
    path: &str,
) -> Result<(), String> {
    if id <= 0 {
        return Err(format!("{path} 必须大于 0"));
    }
    if !allowed_ids.contains(&id) {
        return Err(format!("{path} 引用了不在本次输入候选中的 ID: {id}"));
    }
    Ok(())
}
