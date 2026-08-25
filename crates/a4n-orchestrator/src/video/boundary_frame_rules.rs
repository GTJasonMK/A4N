//! 边界帧编号、提示词链和生成图片匹配规则（无 DB、无 Host）。

use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT, image_generation_prompt_id,
    image_generation_source,
};

use super::{PromptChainFields, prompt_chain_matches, prompt_source_type_matches};

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

pub fn frame_prompt_matches_video_prompt<P: PromptChainFields>(
    video_prompt: &chapter_video_prompt::Model,
    frame_prompt: &P,
) -> bool {
    prompt_chain_matches(
        chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES,
        video_prompt,
        frame_prompt,
    ) && prompt_source_type_matches(
        video_prompt,
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
    ) && prompt_source_type_matches(
        frame_prompt,
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
    )
}

pub fn latest_boundary_frame_image_id<P: PromptChainFields>(
    images: &[generated_image::Reference],
    frame_prompt: &P,
) -> Option<i64> {
    images
        .iter()
        .find(|image| {
            let metadata = image.metadata_json.as_ref();
            image_generation_prompt_id(metadata) == Some(frame_prompt.id())
                && image_generation_source(metadata)
                    == Some(IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT)
        })
        .map(|image| image.id)
}

/// Resolves both boundary images in one pass over the id-descending image list.
pub(in crate::video) fn latest_boundary_frame_image_ids(
    images: &[generated_image::Reference],
    frame_prompt_ids: (Option<i64>, Option<i64>),
) -> (Option<i64>, Option<i64>) {
    let (first_prompt_id, last_prompt_id) = frame_prompt_ids;
    if first_prompt_id.is_none() && last_prompt_id.is_none() {
        return (None, None);
    }

    let mut first_image_id = None;
    let mut last_image_id = None;
    for image in images {
        let metadata = image.metadata_json.as_ref();
        if image_generation_source(metadata)
            != Some(IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT)
        {
            continue;
        }
        let Some(prompt_id) = image_generation_prompt_id(metadata) else {
            continue;
        };
        if first_image_id.is_none() && first_prompt_id == Some(prompt_id) {
            first_image_id = Some(image.id);
        }
        if last_image_id.is_none() && last_prompt_id == Some(prompt_id) {
            last_image_id = Some(image.id);
        }
        if first_image_id.is_some() && last_image_id.is_some() {
            break;
        }
    }
    (first_image_id, last_image_id)
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn image(id: i64, prompt_id: i64, source: &str) -> generated_image::Reference {
        generated_image::Reference {
            id,
            image_type: "video_boundary_frame".to_string(),
            metadata_json: Some(json!({
                "generation_context": {
                    "prompt_id": prompt_id,
                    "source": source,
                }
            })),
        }
    }

    #[test]
    fn boundary_image_pair_keeps_descending_latest_and_shared_prompt_semantics() {
        let images = vec![
            image(50, 11, "other"),
            image(40, 10, IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT),
            image(39, 10, IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT),
            image(30, 11, IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT),
        ];

        assert_eq!(
            latest_boundary_frame_image_ids(&images, (Some(10), Some(11))),
            (Some(40), Some(30))
        );
        assert_eq!(
            latest_boundary_frame_image_ids(&images, (Some(10), Some(10))),
            (Some(40), Some(40))
        );
    }
}
