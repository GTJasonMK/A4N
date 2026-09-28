use serde_json::Value;
use std::collections::HashSet;

pub fn validate_reference_image_ids(image_ids: &[i64]) -> Result<Vec<i64>, String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(image_ids.len());
    for image_id in image_ids.iter().copied() {
        if image_id <= 0 {
            return Err(format!("参考图 ID 必须为正整数：{image_id}"));
        }
        if !seen.insert(image_id) {
            return Err(format!("参考图包含重复 ID：{image_id}"));
        }
        normalized.push(image_id);
    }
    Ok(normalized)
}

pub fn reference_image_ids(value: Option<&Value>) -> Result<Vec<i64>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| "reference_image_ids 必须是数组".to_string())?;
    let mut ids = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = value
            .as_i64()
            .filter(|id| *id > 0)
            .ok_or_else(|| format!("reference_image_ids[{index}] 必须是正整数"))?;
        if ids.contains(&id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
        ids.push(id);
    }
    Ok(ids)
}

pub fn validate_reference_image_id_list(ids: &[i64]) -> Result<(), String> {
    let mut seen = HashSet::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(format!("reference_image_ids[{index}] 必须是正整数"));
        }
        if !seen.insert(id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
    }
    Ok(())
}

pub fn batch_item_reference_image_ids(item: &Value) -> Result<Vec<i64>, String> {
    let Some(value) = item
        .get("reference_image_ids")
        .filter(|value| !value.is_null())
    else {
        return Ok(Vec::new());
    };
    reference_image_ids(Some(value))
}
