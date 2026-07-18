use serde_json::Value;

pub(in crate::actions::video_generation) fn reference_image_ids(
    value: Option<&Value>,
) -> Result<Vec<i64>, String> {
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

pub(super) fn ensure_reference_image_ids_valid(ids: &[i64]) -> Result<(), String> {
    let mut seen = Vec::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(format!("reference_image_ids[{index}] 必须是正整数"));
        }
        if seen.contains(&id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
        seen.push(id);
    }
    Ok(())
}
