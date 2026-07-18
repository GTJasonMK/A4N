use super::super::*;
use crate::video::prompt_json::sanitize_prompt_asset_text;

fn reference_slot_label(slot_index: usize) -> String {
    format!("参考图 {slot_index}")
}

pub(in crate::video) fn reference_slots_for_frame(
    reference_slot_count: usize,
    first_slot_role: Option<&str>,
) -> Value {
    Value::Array(
        (0..reference_slot_count)
            .map(|index| {
                let slot_index = index + 1;
                let role = if slot_index == 1 {
                    first_slot_role.unwrap_or("material_reference")
                } else {
                    "material_reference"
                };
                json!({
                    "slot_index": slot_index,
                    "slot_label": reference_slot_label(slot_index),
                    "role": role,
                    "purpose": if role == "previous_boundary_frame" {
                        "上一帧实际画面连续性约束"
                    } else {
                        "人物、场景、道具或风格一致性约束"
                    }
                })
            })
            .collect(),
    )
}

pub(in crate::video) fn frame_prompt_available_assets_context(
    available_assets: &Value,
) -> Result<Value, AppError> {
    let items = available_assets.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频帧提示词 available_assets 必须是数组".to_string())
    })?;
    Ok(Value::Array(
        items
            .iter()
            .map(frame_prompt_available_asset_context)
            .collect(),
    ))
}

fn frame_prompt_available_asset_context(asset: &Value) -> Value {
    let mut object = serde_json::Map::new();
    for key in [
        "role",
        "label",
        "description",
        "prompt_excerpt",
        "selection_reason",
        "profile_kind",
    ] {
        if let Some(value) = sanitize_prompt_asset_text(asset.get(key)) {
            object.insert(key.to_string(), Value::String(value));
        }
    }
    for key in ["is_primary", "is_favorite"] {
        if let Some(value) = asset.get(key).and_then(Value::as_bool) {
            object.insert(key.to_string(), Value::Bool(value));
        }
    }
    Value::Object(object)
}
pub(in crate::video) fn previous_boundary_frame_asset_context(
    available_assets: &Value,
    image_id: Option<i64>,
) -> Option<Value> {
    let image_id = image_id?;
    available_assets
        .as_array()?
        .iter()
        .find(|asset| asset.get("image_id").and_then(Value::as_i64) == Some(image_id))
        .map(|_| {
            json!({
                "slot_index": 1,
                "slot_label": "参考图 1",
                "usage": "已生成的上一张边界帧画面；后端会把它绑定为参考图 1"
            })
        })
}

pub(in crate::video) fn backend_frame_reference_ids(
    material_reference_image_ids: &[i64],
    first_reference_image_id: Option<i64>,
) -> Vec<i64> {
    let mut ids = Vec::new();
    if let Some(id) = first_reference_image_id {
        ids.push(id);
    }
    for id in material_reference_image_ids.iter().copied() {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

pub(in crate::video) fn available_assets_with_previous_boundary_frame(
    available_assets: &Value,
    previous_image_id: Option<i64>,
    previous_frame: Option<&VideoFramePrompt>,
) -> Value {
    let Some(previous_image_id) = previous_image_id else {
        return available_assets.clone();
    };
    let Some(items) = available_assets.as_array() else {
        return available_assets.clone();
    };
    if items
        .iter()
        .any(|asset| asset.get("image_id").and_then(Value::as_i64) == Some(previous_image_id))
    {
        return available_assets.clone();
    }

    let display_order = items
        .iter()
        .filter_map(|asset| asset.get("display_order").and_then(Value::as_u64))
        .max()
        .unwrap_or(items.len() as u64)
        + 1;
    let mut expanded = items.clone();
    expanded.push(json!({
        "image_id": previous_image_id,
        "display_order": display_order,
        "display_label": format!("参考图 {display_order}"),
        "role": "video_key_frame",
        "label": previous_frame
            .map(|frame| frame.title.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("已生成边界帧"),
        "description": previous_frame.map(|frame| {
            format!(
                "已生成边界帧 #{} 的实际画面。画面状态：{}",
                frame.frame_number,
                frame.scene_summary
            )
        }),
        "prompt_excerpt": previous_frame.map(|frame| frame.frame_prompt.as_str()),
        "selection_reason": "boundary_frame_continuity",
        "profile_kind": Value::Null,
        "profile_id": Value::Null,
        "is_primary": false,
        "is_favorite": false,
    }));
    Value::Array(expanded)
}
