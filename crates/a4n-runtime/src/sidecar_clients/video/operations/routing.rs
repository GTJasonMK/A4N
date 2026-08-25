use a4n_capabilities::A4nCapabilities;
use a4n_core::models::ai::video_gen_config;
#[cfg(feature = "embedded-media-providers")]
use a4n_orchestrator::video::validate_video_config_model;
use a4n_video::{
    VideoGenerationRequest, VideoGenerationResult, VideoJobDeleteOutcome, VideoJobState,
    VideoJobStatus, VideoProvider,
};
#[cfg(feature = "embedded-media-providers")]
use a4n_video_providers::pick_video_provider;
use tokio::sync::mpsc::UnboundedSender;

use crate::state::RuntimeProfile;

use super::super::super::missing_sidecar_capability_message;
use super::super::events::generate_video_via_sidecar;
use super::sidecar::{delete_video_via_sidecar, poll_video_via_sidecar, video_status_via_sidecar};

fn pick_provider_for_config(
    config: &video_gen_config::Model,
) -> Result<Box<dyn VideoProvider>, String> {
    #[cfg(feature = "embedded-media-providers")]
    {
        let params = validate_video_config_model(config)?;
        let api_key = params
            .api_key_encrypted
            .as_deref()
            .ok_or_else(|| "视频生成配置缺少 api_key_encrypted".to_string())?;
        pick_video_provider(&params.backend, &params.base_url, api_key)
            .map_err(|error| error.to_string())
    }
    #[cfg(not(feature = "embedded-media-providers"))]
    {
        let _ = config;
        Err(
            "视频 provider 未启用：请使用 video sidecar，或用 embedded-media-providers feature 构建"
                .to_string(),
        )
    }
}

pub(crate) async fn generate_video(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    config: &video_gen_config::Model,
    request: VideoGenerationRequest,
    provider_idempotency_key_sha256: Option<String>,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    progress_tx: Option<UnboundedSender<String>>,
) -> Result<VideoGenerationResult, String> {
    if profile.uses_sidecars() {
        let client = capabilities
            .video
            .clone()
            .ok_or_else(|| missing_sidecar_capability_message("video"))?;
        return generate_video_via_sidecar(
            client,
            config,
            request,
            provider_idempotency_key_sha256,
            project_id,
            root_project_id,
            workflow_run_id,
            progress_tx,
        )
        .await;
    }

    let provider = pick_provider_for_config(config)?;
    provider
        .generate(request, provider_idempotency_key_sha256.as_deref())
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn poll_video(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    config: &video_gen_config::Model,
    job_id: &str,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
) -> Result<VideoJobState, String> {
    if profile.uses_sidecars() {
        let client = capabilities
            .video
            .clone()
            .ok_or_else(|| missing_sidecar_capability_message("video"))?;
        return poll_video_via_sidecar(
            client,
            config,
            job_id,
            project_id,
            root_project_id,
            workflow_run_id,
        )
        .await;
    }

    let provider = pick_provider_for_config(config)?;
    provider.poll(job_id).await.map_err(|e| e.to_string())
}

pub(crate) async fn video_job_status(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    config: &video_gen_config::Model,
    job_id: &str,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<VideoJobStatus, String> {
    if profile.uses_sidecars() {
        let client = capabilities
            .video
            .clone()
            .ok_or_else(|| missing_sidecar_capability_message("video"))?;
        return video_status_via_sidecar(client, config, job_id, project_id, root_project_id).await;
    }

    let provider = pick_provider_for_config(config)?;
    provider.status(job_id).await.map_err(|e| e.to_string())
}

pub(crate) async fn delete_video_job(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    config: &video_gen_config::Model,
    job_id: &str,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<VideoJobDeleteOutcome, String> {
    if profile.uses_sidecars() {
        let client = capabilities
            .video
            .clone()
            .ok_or_else(|| missing_sidecar_capability_message("video"))?;
        return delete_video_via_sidecar(client, config, job_id, project_id, root_project_id).await;
    }

    let provider = pick_provider_for_config(config)?;
    provider
        .delete(job_id)
        .await
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video_config(api_key: Option<&str>) -> video_gen_config::Model {
        let now = chrono::Utc::now().naive_utc();
        video_gen_config::Model {
            id: 1,
            name: "primary".to_string(),
            backend: "openai-compatible-video".to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            api_key_encrypted: api_key.map(str::to_string),
            model: "video-model".to_string(),
            max_concurrent: 1,
            default_width: 1280,
            default_height: 720,
            default_duration_secs: 5,
            min_duration_secs: 5,
            max_duration_secs: 10,
            duration_mode: "range".to_string(),
            supported_durations_json: None,
            default_fps: 24,
            is_default: true,
            created_at: now,
            updated_at: now,
        }
    }

    #[cfg(feature = "embedded-media-providers")]
    #[test]
    fn configured_video_provider_requires_api_key_at_request_boundary() {
        let Err(error) = pick_provider_for_config(&video_config(None)) else {
            panic!("provider requests still require a key");
        };

        assert!(error.contains("视频生成配置缺少 api_key_encrypted"));
    }

    #[cfg(not(feature = "embedded-media-providers"))]
    #[test]
    fn configured_video_provider_requires_sidecar_without_embedded_feature() {
        let Err(error) = pick_provider_for_config(&video_config(Some("test-key"))) else {
            panic!("minimal build must not construct an embedded video provider");
        };

        assert!(error.contains("请使用 video sidecar"));
    }
}
