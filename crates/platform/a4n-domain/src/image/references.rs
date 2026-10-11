use serde_json::Value;
use std::collections::HashSet;

pub fn validate_reference_image_ids(image_ids: &[i64]) -> Result<Vec<i64>, String> {
    let mut seen = HashSet::with_capacity(image_ids.len());
    let mut normalized = Vec::with_capacity(image_ids.len());
    for (index, image_id) in image_ids.iter().copied().enumerate() {
        validate_reference_image_id(
            &mut seen,
            index,
            image_id,
            ReferenceImageIdErrorStyle::Portrait,
        )?;
        normalized.push(image_id);
    }
    Ok(normalized)
}

#[derive(Clone, Copy)]
enum ReferenceImageIdErrorStyle {
    Portrait,
    Field,
}

fn invalid_reference_image_id_field(index: usize) -> String {
    format!("reference_image_ids[{index}] 必须是正整数")
}

fn validate_reference_image_id(
    seen: &mut HashSet<i64>,
    index: usize,
    id: i64,
    error_style: ReferenceImageIdErrorStyle,
) -> Result<(), String> {
    if id <= 0 {
        return Err(match error_style {
            ReferenceImageIdErrorStyle::Portrait => {
                format!("参考图 ID 必须为正整数：{id}")
            }
            ReferenceImageIdErrorStyle::Field => invalid_reference_image_id_field(index),
        });
    }
    if !seen.insert(id) {
        return Err(match error_style {
            ReferenceImageIdErrorStyle::Portrait => format!("参考图包含重复 ID：{id}"),
            ReferenceImageIdErrorStyle::Field => {
                format!("reference_image_ids 包含重复 ID：{id}")
            }
        });
    }
    Ok(())
}

pub fn reference_image_ids(value: Option<&Value>) -> Result<Vec<i64>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| "reference_image_ids 必须是数组".to_string())?;
    let mut ids = Vec::with_capacity(values.len());
    let mut seen = HashSet::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = value
            .as_i64()
            .ok_or_else(|| invalid_reference_image_id_field(index))?;
        validate_reference_image_id(&mut seen, index, id, ReferenceImageIdErrorStyle::Field)?;
        ids.push(id);
    }
    Ok(ids)
}

pub fn validate_reference_image_id_list(ids: &[i64]) -> Result<(), String> {
    let mut seen = HashSet::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        validate_reference_image_id(&mut seen, index, id, ReferenceImageIdErrorStyle::Field)?;
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
