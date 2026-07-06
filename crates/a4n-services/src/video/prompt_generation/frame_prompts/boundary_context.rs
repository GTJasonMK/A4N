use super::*;
use crate::video::prompt_json::sanitize_prompt_asset_text;

fn frame_context_value(frame: &VideoFramePrompt) -> Value {
    json!({
        "frame_number": frame.frame_number,
        "title": frame.title.as_str(),
        "scene_summary": frame.scene_summary.as_str(),
        "frame_prompt": frame.frame_prompt.as_str(),
        "blocking_state": frame.blocking_state.clone(),
        "lighting_state": frame.lighting_state.clone(),
        "segment_numbers": &frame.segment_numbers
    })
}

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

pub(in crate::video) fn boundary_frame_generation_context(
    segment_plan: &[VideoSegmentPlan],
    completed_frames: &[VideoFramePrompt],
    frame_number: i32,
    previous_frame_image: Option<Value>,
) -> Result<Value, AppError> {
    let expected_segment_numbers =
        boundary_frame_expected_segment_numbers(segment_plan, frame_number)?;
    let required_duration_secs =
        boundary_frame_expected_duration(segment_plan, &expected_segment_numbers)?;
    let previous_segment = if frame_number > 1 {
        segment_plan.get(((frame_number - 2) / 2) as usize)
    } else {
        None
    };
    let current_segment = segment_plan.get(((frame_number - 1) / 2) as usize);
    let next_segment = segment_plan.get(((frame_number + 1) / 2) as usize);
    let previous_frame = completed_frames.last().map(frame_context_value);
    let role = boundary_frame_role(segment_plan, frame_number)?;
    let required_aspect_ratio = current_segment
        .map(|segment| segment.aspect_ratio.as_str())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "边界帧 #{} 无法从片段规划推导 required_aspect_ratio",
                frame_number
            ))
        })?;
    Ok(json!({
        "frame_number": frame_number,
        "total_frame_count": expected_boundary_frame_count(segment_plan),
        "role": role,
        "expected_segment_numbers": expected_segment_numbers,
        "required_duration_secs": required_duration_secs,
        "required_aspect_ratio": required_aspect_ratio,
        "previous_frame": previous_frame,
        "previous_frame_image": previous_frame_image,
        "reference_slot_instruction": if frame_number == 1 {
            "后端提供的 constraints.reference_slots 顺序就是本帧成图输入顺序；frame_prompt 只能按槽位顺序写参考图 1、参考图 2，不得输出 reference_image_ids，也不得写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID。"
        } else {
            "如果 previous_frame_image 存在，后端会把上一帧实际图片绑定为参考图 1；其他素材按 constraints.reference_slots 后续顺序称为参考图 2、参考图 3。不要输出 reference_image_ids，不得写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID。"
        },
        "previous_segment": previous_segment,
        "current_segment": current_segment,
        "next_segment": next_segment,
        "continuity_instruction": if frame_number == 1 {
            "这是第一个片段的首帧，不要编造上一帧；必须建立人物、场景锚点、构图、道具和光源基准。"
        } else if role == "segment_start" {
            "这是当前片段独立首帧；它可以承接上一片段尾帧的时间关系，但不能与上一片段共享同一张帧。必须明确当前片段开场的人物、空间、道具、光线和情绪状态。"
        } else {
            "这是当前片段独立尾帧；必须稳定留下本片段结束时的人物姿态、空间锚点、道具状态、情绪和镜头方向，供后续过渡视频使用。"
        }
    }))
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

pub(super) fn frame_prompt_with_style_template(
    prompt: &str,
    style: &str,
    prompt_output_language: PromptOutputLanguage,
) -> String {
    let style = style.trim();
    if style.is_empty() || prompt.contains(style) {
        return prompt.trim().to_string();
    }
    let style_intro = match prompt_output_language {
        PromptOutputLanguage::ZhCn => "画面风格模板原文",
        PromptOutputLanguage::En => "Verbatim visual style template",
    };
    let style_line = format!("\n{style_intro}: {style}\n");
    let frame_sections = prompt_output_language.frame_sections();
    let style_section = frame_sections[3];
    let next_section = frame_sections[4];
    if let Some(section_start) = prompt.find(style_section) {
        let after_section_start = section_start + style_section.len();
        let after_section = &prompt[after_section_start..];
        if let Some(next_section_start) = after_section.find(next_section) {
            let insert_at = after_section_start + next_section_start;
            let mut value = String::with_capacity(prompt.len() + style_line.len());
            value.push_str(prompt[..insert_at].trim_end());
            value.push_str(&style_line);
            value.push_str(prompt[insert_at..].trim_start());
            return value.trim().to_string();
        }
    }
    format!("{}\n{}{}", prompt.trim(), style_section, style_line)
        .trim()
        .to_string()
}

pub(super) fn apply_boundary_frame_style_template(
    frame: &mut VideoFramePrompt,
    style: &str,
    prompt_output_language: PromptOutputLanguage,
) {
    frame.frame_prompt =
        frame_prompt_with_style_template(&frame.frame_prompt, style, prompt_output_language);
}
