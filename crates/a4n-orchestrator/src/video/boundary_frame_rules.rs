//! 边界帧编号、提示词链和生成图片匹配规则（无 DB、无 Host）。

use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;

use super::{
    image_generation_prompt_id, image_generation_source, prompt_chain_matches,
    prompt_source_type_matches,
};

pub fn boundary_frame_numbers_for_prompt(
    prompt: &chapter_video_prompt::Model,
) -> Result<(i32, i32), String> {
    let metadata = prompt.metadata_json.as_ref().ok_or_else(|| {
        format!(
            "视频提示词 #{} 缺少结构化元数据，请重新生成视频提示词后再生成视频",
            prompt.id
        )
    })?;
    let video_unit = metadata
        .get("video_unit")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} 缺少 metadata_json.video_unit，请重新生成视频提示词后再生成视频",
                prompt.id
            )
        })?;
    let first = required_video_unit_frame_number(
        prompt,
        video_unit,
        "start_frame_number",
        "起始边界帧编号",
    )?;
    let last =
        required_video_unit_frame_number(prompt, video_unit, "end_frame_number", "结束边界帧编号")?;
    Ok((first, last))
}

pub fn frame_prompt_matches_video_prompt(
    video_prompt: &chapter_video_prompt::Model,
    frame_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches("boundary_frames", video_prompt, frame_prompt)
        && prompt_source_type_matches(video_prompt, "frame_segment_plan")
        && prompt_source_type_matches(frame_prompt, "frame_segment_plan")
}

pub fn latest_boundary_frame_image_id(
    images: &[generated_image::Model],
    frame_prompt: &chapter_video_prompt::Model,
) -> Option<i64> {
    images
        .iter()
        .find(|image| {
            let metadata = image.metadata_json.as_ref();
            image_generation_prompt_id(metadata) == Some(frame_prompt.id)
                && image_generation_source(metadata) == Some("video_boundary_frame_prompt")
        })
        .map(|image| image.id)
}

fn required_video_unit_frame_number(
    prompt: &chapter_video_prompt::Model,
    video_unit: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    label: &str,
) -> Result<i32, String> {
    let value = video_unit
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} metadata_json.video_unit.{key} 缺少有效{label}，请重新生成视频提示词后再生成视频",
                prompt.id
            )
        })?;
    i32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} metadata_json.video_unit.{key} 必须是正整数，实际为 {value}",
                prompt.id
            )
        })
}
