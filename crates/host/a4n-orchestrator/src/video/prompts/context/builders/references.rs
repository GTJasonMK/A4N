use std::collections::HashSet;

use serde_json::{Map, Value, json};

use a4n_error::AppError;

use crate::video::prompts::json::sanitize_prompt_asset_text;

use super::VideoPromptContractMode;

pub(super) fn frame_reference_slots(
    reference_ids: &Value,
    first_slot_role: Option<&str>,
) -> Result<Value, AppError> {
    let reference_ids = reference_ids.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频帧提示词 reference_ids 必须是数组".to_string())
    })?;
    if first_slot_role.is_some() && reference_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频帧提示词上一帧参考图已存在，但 reference_ids 没有提供对应的第 1 个参考槽位"
                .to_string(),
        ));
    }
    let mut seen_ids = HashSet::with_capacity(reference_ids.len());
    let mut slots = Vec::with_capacity(reference_ids.len());
    for (index, value) in reference_ids.iter().enumerate() {
        let image_id = value.as_i64().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频帧提示词 reference_ids 第 {} 项必须是图片 ID 整数",
                index + 1
            ))
        })?;
        if image_id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "视频帧提示词 reference_ids 第 {} 项必须是正数图片 ID",
                index + 1
            )));
        }
        if !seen_ids.insert(image_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频帧提示词 reference_ids 包含重复图片 ID：{image_id}"
            )));
        }
        let slot_index = index + 1;
        let role = if slot_index == 1 {
            first_slot_role.unwrap_or("material_reference")
        } else {
            "material_reference"
        };
        let purpose = if role == "previous_boundary_frame" {
            "上一帧实际画面连续性约束"
        } else {
            "人物、场景、道具或风格一致性约束"
        };
        slots.push(json!({
            "slot_index": slot_index,
            "slot_label": format!("参考图 {slot_index}"),
            "role": role,
            "purpose": purpose
        }));
    }
    Ok(Value::Array(slots))
}

pub(super) fn frame_reference_slot_order_rule() -> &'static str {
    "frame_prompt 自然语言中的参考图编号必须由后端提供的 constraints.reference_slots 顺序决定：第一项写“参考图 1”，第二项写“参考图 2”。不要输出 reference_image_ids，不要把 available_assets 的候选序号、候选素材标签、字段名或内部编号当成参考图编号；frame_prompt 和 negative_prompt 正文不得出现 image_id、image id、图片 ID、参考图 ID、available_assets、manual_library_asset_* 或任何原始图片数字 ID。"
}

pub(super) fn video_prompt_reference_slots(
    reference_ids: &Value,
    mode: VideoPromptContractMode,
) -> Result<Value, AppError> {
    let reference_ids = reference_ids.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 reference_ids 必须是数组".to_string())
    })?;
    let mut seen = HashSet::with_capacity(reference_ids.len());
    let mut slots = Vec::with_capacity(reference_ids.len());
    for (index, value) in reference_ids.iter().enumerate() {
        let image_id = value.as_i64().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 reference_ids 第 {} 项必须是图片 ID 整数",
                index + 1
            ))
        })?;
        if image_id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 reference_ids 第 {} 项必须是正数图片 ID",
                index + 1
            )));
        }
        if !seen.insert(image_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 reference_ids 包含重复图片 ID：{image_id}"
            )));
        }
        let slot_index = index + 1;
        let slot_label = match mode {
            VideoPromptContractMode::Api => format!("参考图 {slot_index}"),
            VideoPromptContractMode::Chat => format!("第 {slot_index} 张图片"),
        };
        slots.push(json!({
            "slot_index": slot_index,
            "slot_label": slot_label,
            "purpose": match mode {
                VideoPromptContractMode::Api => "人物、场景、道具、风格或边界帧连续性约束",
                VideoPromptContractMode::Chat => "外部 Chat 页面按该顺序上传和描述的输入图片",
            }
        }));
    }
    Ok(Value::Array(slots))
}

pub(super) fn reference_asset_summaries(available_assets: &Value) -> Result<Value, AppError> {
    let items = available_assets.as_array().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 reference_asset_summaries 来源必须是数组".to_string())
    })?;
    Ok(Value::Array(
        items.iter().map(reference_asset_summary).collect(),
    ))
}

pub(in crate::video) fn reference_asset_summary(asset: &Value) -> Value {
    let mut object = Map::new();
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
