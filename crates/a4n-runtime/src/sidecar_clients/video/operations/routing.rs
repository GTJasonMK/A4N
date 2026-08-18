use a4n_capabilities::A4nCapabilities;
use a4n_core::models::ai::video_gen_config;
use a4n_orchestrator::video::pick_provider_for_config;
use a4n_video::{
    VideoGenerationRequest, VideoGenerationResult, VideoJobDeleteOutcome, VideoJobState,
    VideoJobStatus,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::state::RuntimeProfile;

use super::super::super::missing_sidecar_capability_message;
use super::super::events::generate_video_via_sidecar;
use super::sidecar::{delete_video_via_sidecar, poll_video_via_sidecar, video_status_via_sidecar};

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
