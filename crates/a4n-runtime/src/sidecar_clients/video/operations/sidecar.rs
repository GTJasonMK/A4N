use std::sync::Arc;

use a4n_capabilities::{A4nCapabilities, VideoClient};
use a4n_core::models::ai::video_gen_config;
use a4n_orchestrator::video::validate_video_config_model;
use a4n_video::{VideoGenerationRequest, VideoJobDeleteOutcome, VideoJobState, VideoJobStatus};
use serde_json::Value;

use super::super::super::{
    SidecarRequestContext, global_provider_service_request, missing_sidecar_capability_message,
    response_payload, service_error_string, service_request_with_context_provider_secret_policy,
    sidecar_context_for_workflow_operation_with_root,
};
use super::super::contracts::{
    VideoDeleteOutput, VideoJobOutput, VideoPollPayload, VideoStatusOutput,
    video_generation_result, video_provider_payload,
};

pub(crate) async fn preview_video_provider_request(
    capabilities: &A4nCapabilities,
    config: &video_gen_config::Model,
    request: VideoGenerationRequest,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<Value, String> {
    let client = capabilities
        .video
        .clone()
        .ok_or_else(|| missing_sidecar_capability_message("video"))?;
    let payload = video_provider_payload(config, request, None, project_id)?;
    let response = client
        .preview_request(service_request_with_context_provider_secret_policy(
            SidecarRequestContext::current_global().with_project_scope(project_id, root_project_id),
            "video",
            a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
            payload,
        )?)
        .await
        .map_err(service_error_string)?;
    let preview: Value = response_payload(response, "video")?;
    if preview.is_null() {
        return Err("video sidecar request preview 为空".to_string());
    }
    Ok(preview)
}

pub(super) async fn probe_video_via_sidecar(
    client: Arc<dyn VideoClient>,
    config: &video_gen_config::Model,
) -> Result<(), String> {
    let params = validate_video_config_model(config)?;
    let request = VideoGenerationRequest {
        prompt: "test connectivity".to_string(),
        negative_prompt: None,
        width: params.default_width,
        height: params.default_height,
        duration_secs: params.default_duration_secs,
        fps: params.default_fps,
        model: params.model.clone(),
        media: a4n_video::VideoMediaInput::None,
    };
    let payload = video_provider_payload(config, request, None, None)?;
    let response = client
        .probe(global_provider_service_request(
            "video",
            a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
            payload,
        )?)
        .await
        .map_err(service_error_string)?;
    let _: Value = response_payload(response, "video")?;
    Ok(())
}

pub(super) async fn poll_video_via_sidecar(
    client: Arc<dyn VideoClient>,
    config: &video_gen_config::Model,
    job_id: &str,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
) -> Result<VideoJobState, String> {
    let params = validate_video_config_model(config)?;
    let api_key = params
        .api_key_encrypted
        .ok_or_else(|| "视频生成配置缺少 api_key_encrypted".to_string())?;
    let payload = VideoPollPayload {
        backend: params.backend,
        base_url: params.base_url,
        api_key,
        project_id,
        job_id: job_id.to_string(),
    };
    let response = client
        .poll(service_request_with_context_provider_secret_policy(
            sidecar_context_for_workflow_operation_with_root(
                project_id,
                root_project_id,
                workflow_run_id,
                "video.poll",
            )?,
            "video",
            a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
            payload,
        )?)
        .await
        .map_err(service_error_string)?;
    response_payload(response, "video").and_then(|output: VideoJobOutput| match output {
        VideoJobOutput::Pending => Ok(VideoJobState::Pending),
        VideoJobOutput::Running => Ok(VideoJobState::Running),
        VideoJobOutput::Completed { result } => {
            video_generation_result(result).map(VideoJobState::Completed)
        }
        VideoJobOutput::Failed { message } => Ok(VideoJobState::Failed(message)),
    })
}

pub(super) async fn video_status_via_sidecar(
    client: Arc<dyn VideoClient>,
    config: &video_gen_config::Model,
    job_id: &str,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<VideoJobStatus, String> {
    let params = validate_video_config_model(config)?;
    let api_key = params
        .api_key_encrypted
        .ok_or_else(|| "视频生成配置缺少 api_key_encrypted".to_string())?;
    let payload = VideoPollPayload {
        backend: params.backend,
        base_url: params.base_url,
        api_key,
        project_id,
        job_id: job_id.to_string(),
    };
    let response = client
        .status(service_request_with_context_provider_secret_policy(
            SidecarRequestContext::current_global().with_project_scope(project_id, root_project_id),
            "video",
            a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
            payload,
        )?)
        .await
        .map_err(service_error_string)?;
    response_payload(response, "video").map(|output: VideoStatusOutput| match output {
        VideoStatusOutput::Pending => VideoJobStatus::Pending,
        VideoStatusOutput::Running => VideoJobStatus::Running,
        VideoStatusOutput::Completed => VideoJobStatus::Completed,
        VideoStatusOutput::Failed { message } => VideoJobStatus::Failed(message),
        VideoStatusOutput::Cancelled => VideoJobStatus::Cancelled,
        VideoStatusOutput::NotFound => VideoJobStatus::NotFound,
    })
}

pub(super) async fn delete_video_via_sidecar(
    client: Arc<dyn VideoClient>,
    config: &video_gen_config::Model,
    job_id: &str,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<VideoJobDeleteOutcome, String> {
    let params = validate_video_config_model(config)?;
    let api_key = params
        .api_key_encrypted
        .ok_or_else(|| "视频生成配置缺少 api_key_encrypted".to_string())?;
    let payload = VideoPollPayload {
        backend: params.backend,
        base_url: params.base_url,
        api_key,
        project_id,
        job_id: job_id.to_string(),
    };
    let response = client
        .delete(service_request_with_context_provider_secret_policy(
            SidecarRequestContext::current_global().with_project_scope(project_id, root_project_id),
            "video",
            a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
            payload,
        )?)
        .await
        .map_err(service_error_string)?;
    response_payload(response, "video").map(|output: VideoDeleteOutput| match output {
        VideoDeleteOutput::Deleted => VideoJobDeleteOutcome::Deleted,
        VideoDeleteOutput::NotFound => VideoJobDeleteOutcome::NotFound,
    })
}
