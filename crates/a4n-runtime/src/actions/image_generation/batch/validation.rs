use serde_json::Value;

use super::contracts::BATCH_IMAGE_ITEM_FIELDS;

pub(super) fn ensure_batch_image_item_shape(index: usize, item: &Value) -> Result<(), String> {
    let object = item
        .as_object()
        .ok_or_else(|| format!("第 {} 项必须是 JSON 对象", index + 1))?;
    for key in object.keys() {
        if !BATCH_IMAGE_ITEM_FIELDS.contains(&key.as_str()) {
            return Err(format!("第 {} 项包含未知字段：{key}", index + 1));
        }
    }
    optional_i64_field(item, "chapter_id")?;
    optional_i64_field(item, "config_id")?;
    optional_string_field(item, "panel_id")?;
    optional_string_field(item, "aspect_ratio")?;
    Ok(())
}

pub(super) fn optional_i64_field(item: &Value, field: &str) -> Result<Option<i64>, String> {
    let Some(value) = item.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_i64()
        .map(Some)
        .ok_or_else(|| format!("{field} 必须是整数"))
}

pub(super) fn optional_string_field<'a>(
    item: &'a Value,
    field: &str,
) -> Result<Option<&'a str>, String> {
    let Some(value) = item.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| format!("{field} 必须是字符串"))
}
