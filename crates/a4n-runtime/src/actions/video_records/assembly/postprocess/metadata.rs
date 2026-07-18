use super::super::clips::optional_positive_i64;
use super::super::contracts::PostprocessEditMetadata;
use super::range::validate_postprocess_operation;
use a4n_core::models::ai::generated_video;
use serde_json::Value;

pub(in crate::actions::video_records) fn parse_postprocess_edit_metadata(
    video: &generated_video::Model,
) -> Result<PostprocessEditMetadata, String> {
    let metadata = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("postprocess_edit"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "后处理编辑视频 #{} 缺少 postprocess_edit metadata",
                video.id
            )
        })?;
    let push_video_id = required_positive_i64(metadata.get("push_video_id")).map_err(|err| {
        format!(
            "后处理编辑视频 #{} 的 postprocess_edit.push_video_id {err}",
            video.id
        )
    })?;
    let base_video_id = required_positive_i64(metadata.get("base_video_id")).map_err(|err| {
        format!(
            "后处理编辑视频 #{} 的 postprocess_edit.base_video_id {err}",
            video.id
        )
    })?;
    let _operation = metadata
        .get("operation")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "后处理编辑视频 #{} 缺少 postprocess_edit.operation",
                video.id
            )
        })
        .and_then(|value| validate_postprocess_operation(value).map(str::to_string))?;
    let _source_video_id =
        optional_positive_i64(metadata.get("source_video_id")).map_err(|err| {
            format!(
                "后处理编辑视频 #{} 的 postprocess_edit.source_video_id {err}",
                video.id
            )
        })?;
    Ok(PostprocessEditMetadata {
        push_video_id,
        base_video_id,
    })
}

fn required_positive_i64(value: Option<&Value>) -> Result<i64, String> {
    value
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or_else(|| "必须是正整数".to_string())
}
