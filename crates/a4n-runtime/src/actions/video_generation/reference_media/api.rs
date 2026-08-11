use super::super::provider_host::RuntimeVideoProviderHostPort;
use super::super::*;

pub async fn preview_video_provider_request_by_video_id(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
) -> Result<ModelRequestPreview, String> {
    let video = state
        .orchestrator()
        .generated_video_by_id(video_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| "视频记录不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, video.project_id).await?;
    resolve_project_chapter_video_source(
        state,
        project_id,
        chapter_id,
        "预览历史视频 provider 请求",
        false,
    )
    .await?;
    if video.project_id != project_id || video.chapter_id != Some(chapter_id) {
        return Err("视频记录不属于该章节".to_string());
    }
    validate_video_provider_request_snapshot_owner(&video)?;
    model_request_preview_from_provider_snapshot(
        &video,
        "video_provider_request_record",
        "历史视频模型请求",
    )
}

pub async fn preview_reference_media_video_request(
    state: &AppState,
    input: GenerateReferenceMediaVideoInput,
) -> Result<ModelRequestPreview, String> {
    let host_port = RuntimeVideoProviderHostPort::new(state);
    let preview = state
        .orchestrator()
        .preview_reference_media_video_request_validated(&host_port, input)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(model_request_preview(
        "reference_media_video_generation",
        "参考媒体视频模型请求",
        vec![provider_request_step(
            "reference_media_video",
            "参考媒体视频 provider 请求",
            "video_provider_request",
            Some("reference_media_video".to_string()),
            "ready",
            Some(preview.note),
            &preview.request,
        )?],
    ))
}
