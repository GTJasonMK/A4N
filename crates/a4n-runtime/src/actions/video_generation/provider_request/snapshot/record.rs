use a4n_core::{
    models::{ai::generated_video, chapter::chapter_video_prompt},
    video_execution::VideoSegmentWorkflowModeV1,
};
use serde_json::Value;

use super::contract::{optional_snapshot_string, required_snapshot_string};
use crate::actions::request_preview::{
    ModelRequestPreview, model_request_preview, provider_request_step,
};
use crate::actions::video_generation::common::PROVIDER_REQUEST_SNAPSHOT_METADATA_KEY;

pub(in crate::actions::video_generation) fn model_request_preview_from_provider_snapshot(
    video: &generated_video::Model,
    workflow: &str,
    title: &str,
) -> Result<ModelRequestPreview, String> {
    let metadata = video
        .metadata_json
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频 #{} 缺少 metadata_json，无法直接预览生成时构造的 provider 请求",
                video.id
            )
        })?;
    let snapshot = metadata
        .get(PROVIDER_REQUEST_SNAPSHOT_METADATA_KEY)
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频 #{} 缺少 provider 请求快照，无法直接预览生成时构造的请求；请重新生成该视频后再查看历史请求",
                video.id
            )
        })?;
    if snapshot.get("schema_version").and_then(Value::as_i64) != Some(1) {
        return Err(format!(
            "视频 #{} provider 请求快照版本无效，无法直接预览",
            video.id
        ));
    }

    let key = required_snapshot_string(snapshot, "key", video.id)?;
    let step_title = required_snapshot_string(snapshot, "title", video.id)?;
    let kind = required_snapshot_string(snapshot, "kind", video.id)?;
    let state = required_snapshot_string(snapshot, "state", video.id)?;
    let prompt_name = optional_snapshot_string(snapshot, "prompt_name", video.id)?;
    let note = snapshot_note_for_video_record(
        optional_snapshot_string(snapshot, "note", video.id)?,
        video,
    );
    let request = snapshot
        .get("request")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| {
            format!(
                "视频 #{} provider 请求快照缺少 request 对象，无法直接预览",
                video.id
            )
        })?;

    Ok(model_request_preview(
        workflow,
        title,
        vec![provider_request_step(
            key,
            step_title,
            kind,
            prompt_name,
            state,
            note,
            &request,
        )?],
    ))
}

fn snapshot_note_for_video_record(
    note: Option<String>,
    video: &generated_video::Model,
) -> Option<String> {
    let status = video.status.trim();
    let status_note = match status {
        generated_video::STATUS_COMPLETED => None,
        generated_video::STATUS_RUNNING | generated_video::STATUS_PENDING => Some(format!(
            "视频当前状态为 {status}；下方快照是生成流程已经构造出的 provider 请求，任务可能仍在处理中"
        )),
        generated_video::STATUS_FAILED | generated_video::STATUS_CANCELLED => Some(format!(
            "视频当前状态为 {status}；无法仅凭该状态确认 provider 是否实际接收请求，下方展示的是生成流程构造出的 provider 请求快照"
        )),
        other if !other.is_empty() => Some(format!(
            "视频当前状态为 {other}；下方展示的是生成流程构造出的 provider 请求快照"
        )),
        _ => Some("视频状态为空；下方展示的是生成流程构造出的 provider 请求快照".to_string()),
    };
    match (note, status_note) {
        (Some(note), Some(status_note)) if !note.trim().is_empty() => {
            Some(format!("{}；{}", note.trim(), status_note))
        }
        (Some(note), _) if !note.trim().is_empty() => Some(note),
        (_, Some(status_note)) => Some(status_note),
        _ => None,
    }
}

pub(in crate::actions::video_generation) fn validate_video_provider_request_snapshot_owner(
    video: &generated_video::Model,
) -> Result<(), String> {
    match (video.video_type.as_str(), video.workflow_mode.as_deref()) {
        (
            generated_video::VIDEO_TYPE_CHAPTER,
            Some(
                chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
                    | chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN
                    | chapter_video_prompt::WORKFLOW_MODE_STORYBOARD
            ),
        ) => Ok(()),
        (generated_video::VIDEO_TYPE_REFERENCE_MEDIA, Some(mode))
            if mode == VideoSegmentWorkflowModeV1::ReferenceMedia.as_str() => Ok(()),
        (generated_video::VIDEO_TYPE_CHAPTER, Some(mode)) => Err(format!(
            "视频 #{} 的 workflow_mode={} 与 chapter_video 类型不匹配，无法直接预览 provider 请求",
            video.id, mode
        )),
        (generated_video::VIDEO_TYPE_REFERENCE_MEDIA, Some(mode)) => Err(format!(
            "视频 #{} 的 workflow_mode={} 与 reference_media_video 类型不匹配，无法直接预览 provider 请求",
            video.id, mode
        )),
        (
            generated_video::VIDEO_TYPE_CHAPTER | generated_video::VIDEO_TYPE_REFERENCE_MEDIA,
            None,
        ) => Err(format!(
            "视频 #{} 缺少 workflow_mode，无法直接预览 provider 请求",
            video.id
        )),
        _ => Err("裁剪片段没有独立的视频模型 provider 请求".to_string()),
    }
}
