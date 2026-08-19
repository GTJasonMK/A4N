use serde_json::{Value, json};

use crate::video::VideoDbError;
use a4n_core::models::chapter::chapter_video_prompt;

pub fn sync_chapter_video_prompt_metadata_after_update(
    prompt_type: &str,
    metadata_json: Option<Value>,
    visual_prompt: &str,
    negative_prompt: Option<&str>,
    duration_secs: i32,
    reference_image_ids_json: Option<&Value>,
) -> Result<Option<Value>, VideoDbError> {
    if prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO {
        let metadata = metadata_json.ok_or_else(|| {
            VideoDbError::InvalidArgument(
                "视频提示词缺少 video_execution_identity，不能编辑；请重新生成该提示词".to_string(),
            )
        })?;
        if metadata.get("source_prompt_type").and_then(Value::as_str) == Some("inserted_segment") {
            let recipe =
                serde_json::from_value::<a4n_core::video_execution::VideoSegmentExecutionRecipeV1>(
                    metadata.get("segment_recipe").cloned().ok_or_else(|| {
                        VideoDbError::InvalidArgument(
                            "插入片段提示词缺少 segment_recipe，不能编辑".to_string(),
                        )
                    })?,
                )
                .map_err(|error| {
                    VideoDbError::InvalidArgument(format!(
                        "插入片段提示词 segment_recipe 无效，不能编辑：{error}"
                    ))
                })?;
            if duration_secs != recipe.duration_secs {
                return Err(VideoDbError::InvalidArgument(
                    "插入片段时长由 segment_recipe 管理，请修改执行方案后重新生成提示词"
                        .to_string(),
                ));
            }
            return Ok(Some(metadata));
        }
        let mut metadata = metadata;
        let identity = metadata
            .as_object_mut()
            .and_then(|root| root.get_mut("video_execution_identity"))
            .and_then(Value::as_object_mut)
            .ok_or_else(|| {
                VideoDbError::InvalidArgument(
                    "视频提示词 metadata_json 缺少 video_execution_identity，不能编辑；请重新生成该提示词"
                        .to_string(),
                )
            })?;
        identity.insert("duration_secs".to_string(), Value::from(duration_secs));
        return Ok(Some(metadata));
    }
    let Some((metadata_key, label)) = frame_prompt_metadata_key(prompt_type) else {
        return Ok(metadata_json);
    };
    let Some(mut metadata) = metadata_json else {
        return Err(VideoDbError::InvalidArgument(format!(
            "{label}缺少结构化 metadata_json，不能保存编辑后继续用于下游步骤；请重新生成该提示词"
        )));
    };
    let reference_image_ids =
        reference_ids_from_value(reference_image_ids_json).ok_or_else(|| {
            VideoDbError::InvalidArgument(format!(
                "{label} reference_image_ids_json 必须是图片 ID 数组，不能保存编辑"
            ))
        })?;
    let Some(root) = metadata.as_object_mut() else {
        return Err(VideoDbError::InvalidArgument(format!(
            "{label} metadata_json 必须是对象，不能保存编辑后继续用于下游步骤"
        )));
    };
    let Some(frame) = root.get_mut(metadata_key).and_then(Value::as_object_mut) else {
        return Err(VideoDbError::InvalidArgument(format!(
            "{label} metadata_json 缺少 {metadata_key} 对象，不能保存编辑后继续用于下游步骤"
        )));
    };
    frame.insert(
        "frame_prompt".to_string(),
        Value::String(visual_prompt.trim().to_string()),
    );
    frame.insert(
        "negative_prompt".to_string(),
        Value::String(negative_prompt.unwrap_or_default().trim().to_string()),
    );
    frame.insert("duration_secs".to_string(), Value::from(duration_secs));
    frame.insert(
        "reference_image_ids".to_string(),
        json!(reference_image_ids),
    );
    Ok(Some(metadata))
}

fn frame_prompt_metadata_key(prompt_type: &str) -> Option<(&'static str, &'static str)> {
    match prompt_type {
        chapter_video_prompt::PROMPT_TYPE_FRAME
        | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME => {
            Some(("boundary_frame", "边界帧提示词"))
        }
        chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME => Some(("first_frame", "首帧提示词")),
        chapter_video_prompt::PROMPT_TYPE_STORYBOARD => Some(("storyboard_frame", "分镜提示词")),
        _ => None,
    }
}

fn reference_ids_from_value(value: Option<&Value>) -> Option<Vec<i64>> {
    value?
        .as_array()?
        .iter()
        .map(Value::as_i64)
        .collect::<Option<Vec<_>>>()
}
