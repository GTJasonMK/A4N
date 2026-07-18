use serde_json::Value;

use super::super::super::json_utils::required_i32_at;
use super::super::super::sync_types::CharacterSyncContext;

pub(super) fn ensure_stage_anchor_exists(
    item: &Value,
    label: &str,
    index: usize,
    anchor_field: &str,
    context: &CharacterSyncContext,
) -> Result<(), String> {
    let chapter_number = required_i32_at(
        item,
        "chapter_number",
        &format!("{label}[{index}].chapter_number"),
    )?;
    let anchor_id = item
        .get(anchor_field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}[{index}].{anchor_field} 必须是 JSON 整数数字"))?;
    if anchor_id <= 0 {
        return Err(format!("{label}[{index}].{anchor_field} 必须大于 0"));
    }
    let chapter = context
        .chapter_data
        .iter()
        .find(|chapter| {
            chapter
                .get("chapter_number")
                .and_then(Value::as_i64)
                .is_some_and(|value| value == i64::from(chapter_number))
        })
        .ok_or_else(|| format!("{label}[{index}] 缺少可核对证据的第 {chapter_number} 章正文"))?;
    let exists = chapter
        .get("text_anchors")
        .and_then(Value::as_array)
        .is_some_and(|anchors| {
            anchors.iter().any(|anchor| {
                anchor
                    .get("id")
                    .and_then(Value::as_i64)
                    .is_some_and(|value| value == anchor_id)
            })
        });
    if !exists {
        return Err(format!(
            "{label}[{index}].{anchor_field} 不在第 {chapter_number} 章 text_anchors 中：{anchor_id}"
        ));
    }
    Ok(())
}
