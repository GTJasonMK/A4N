use a4n_core::models::ai::{generated_image, generated_video};
use serde_json::Value;

pub(in crate::actions::video_generation::media) fn tail_frame_image_belongs_to_video(
    image: &generated_image::Model,
    video_id: i64,
) -> bool {
    let Some(context) = image
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("generation_context"))
    else {
        return false;
    };
    context
        .get("source")
        .and_then(Value::as_str)
        .is_some_and(|source| source.trim() == "video_tail_frame_capture")
        && context.get("video_id").and_then(Value::as_i64) == Some(video_id)
}

pub(in crate::actions::video_generation::media) fn video_shot_number(
    video: &generated_video::Model,
) -> Result<i32, String> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("视频 #{} 缺少有效 shot_number，无法采集真实尾帧", video.id))
}

pub(super) fn video_source_content_hash(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("source"))
        .and_then(|source| source.get("content_sha256"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}
