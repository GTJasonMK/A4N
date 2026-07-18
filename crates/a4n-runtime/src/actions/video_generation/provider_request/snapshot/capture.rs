use a4n_video::VideoGenerationRequest;
use serde_json::Value;

use super::super::video_provider_preview_payload;
use super::contract::provider_request_snapshot_value;
use crate::AppState;
use crate::actions::video_generation::ChapterVideoProviderRequestContext;
use crate::actions::video_generation::media;

pub(in crate::actions::video_generation) async fn chapter_video_provider_request_snapshot_for_state(
    state: &AppState,
    project_id: i64,
    root_project_id: Option<i64>,
    context: &ChapterVideoProviderRequestContext,
) -> Result<Value, String> {
    if !state.runtime_profile.uses_sidecars() {
        return chapter_video_provider_request_snapshot(context);
    }

    let request = sanitized_chapter_video_request_for_snapshot(context)?;
    let provider_request = crate::sidecar_clients::preview_video_provider_request(
        state.capabilities(),
        &context.cfg,
        request,
        Some(project_id),
        root_project_id,
    )
    .await?;
    let note = match context.provider_input_note.as_deref() {
        Some(note) => format!("生成时由 video sidecar 生成真实 provider 请求体；{note}"),
        None => "生成时由 video sidecar 生成真实 provider 请求体".to_string(),
    };
    Ok(with_video_execution_identity(
        provider_request_snapshot_value(
            "chapter_video",
            "章节视频 provider 请求",
            "video_provider_request",
            Some("chapter_video_prompt"),
            Some(note),
            provider_request,
        ),
        context,
    ))
}

pub(in crate::actions::video_generation) fn chapter_video_provider_request_snapshot(
    context: &ChapterVideoProviderRequestContext,
) -> Result<Value, String> {
    let mut snapshot_context = context.clone();
    snapshot_context.request = sanitized_chapter_video_request_for_snapshot(context)?;
    let (request_kind, request_title, request_note, request) =
        video_provider_preview_payload(&snapshot_context)?;
    Ok(with_video_execution_identity(
        provider_request_snapshot_value(
            "chapter_video",
            request_title,
            request_kind,
            Some("chapter_video_prompt"),
            Some(request_note),
            request,
        ),
        context,
    ))
}

fn with_video_execution_identity(
    mut snapshot: Value,
    context: &ChapterVideoProviderRequestContext,
) -> Value {
    let Some(object) = snapshot.as_object_mut() else {
        return snapshot;
    };
    object.insert(
        "video_execution_identity".to_string(),
        context.video_execution_identity.clone(),
    );
    object.insert(
        "source_prompt_type".to_string(),
        context
            .source_prompt_type
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    object.insert(
        "workflow_mode".to_string(),
        Value::String(context.workflow_mode.clone()),
    );
    snapshot
}

fn sanitized_chapter_video_request_for_snapshot(
    context: &ChapterVideoProviderRequestContext,
) -> Result<VideoGenerationRequest, String> {
    let reference_ids = media::reference_image_ids(context.provider_reference_image_ids.as_ref())?;
    let first_frame_image_url = context
        .boundary_frame_ids
        .first
        .filter(|_| context.request.first_frame_image_url.is_some())
        .map(|id| historical_image_placeholder_media_url(id, "首帧"));
    let last_frame_image_url = context
        .boundary_frame_ids
        .last
        .filter(|_| context.request.last_frame_image_url.is_some())
        .map(|id| historical_image_placeholder_media_url(id, "尾帧"));
    let reference_image_urls = reference_ids
        .into_iter()
        .enumerate()
        .take(context.request.reference_image_urls.len())
        .map(|(index, id)| {
            let role = if context.workflow_mode == "storyboard" && index == 0 {
                "分镜图"
            } else {
                "参考图"
            };
            historical_image_placeholder_media_url(id, role)
        })
        .collect();

    Ok(VideoGenerationRequest {
        prompt: context.request.prompt.clone(),
        negative_prompt: context.request.negative_prompt.clone(),
        width: context.request.width,
        height: context.request.height,
        duration_secs: context.request.duration_secs,
        fps: context.request.fps,
        model: context.request.model.clone(),
        first_frame_image_url,
        last_frame_image_url,
        reference_image_urls,
        reference_media: None,
    })
}

fn historical_image_placeholder_media_url(id: i64, role: &str) -> String {
    format!(
        "https://example.invalid/a4n-media/<omitted for historical provider request snapshot: {role} generated_image #{id}; media input was omitted from snapshot>"
    )
}
