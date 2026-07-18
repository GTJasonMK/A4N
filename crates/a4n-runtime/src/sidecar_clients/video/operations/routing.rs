use a4n_capabilities::A4nCapabilities;
use a4n_core::models::ai::video_gen_config;
use a4n_model_discovery::ModelDiscoveryResult;
use a4n_orchestrator::video::pick_provider_for_config;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult, VideoJobState};
use tokio::sync::mpsc::UnboundedSender;

use crate::state::RuntimeProfile;

use super::super::super::{
    global_provider_service_request, missing_sidecar_capability_message, model_discovery_payload,
    response_payload, service_error_string,
};
use super::super::events::generate_video_via_sidecar;
use super::sidecar::{poll_video_via_sidecar, probe_video_via_sidecar};

pub(crate) async fn generate_video(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    config: &video_gen_config::Model,
    request: VideoGenerationRequest,
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
            project_id,
            root_project_id,
            workflow_run_id,
            progress_tx,
        )
        .await;
    }

    let provider = pick_provider_for_config(config)?;
    provider.generate(request).await.map_err(|e| e.to_string())
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

pub(crate) async fn probe_video_config(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    config: &video_gen_config::Model,
) -> Result<(), String> {
    if profile.uses_sidecars() {
        let client = capabilities
            .video
            .clone()
            .ok_or_else(|| missing_sidecar_capability_message("video"))?;
        return probe_video_via_sidecar(client, config).await;
    }

    pick_provider_for_config(config)?;
    Ok(())
}

pub(crate) async fn discover_video_models(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<ModelDiscoveryResult, String> {
    if profile.uses_sidecars() {
        let client = capabilities
            .video
            .clone()
            .ok_or_else(|| "video sidecar capability 未配置，无法探测模型列表".to_string())?;
        let response = client
            .discover_models(global_provider_service_request(
                "video",
                a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
                model_discovery_payload(provider, base_url, api_key),
            )?)
            .await
            .map_err(service_error_string)?;
        return response_payload(response, "video");
    }

    a4n_model_discovery::discover_models(provider, base_url, api_key).await
}
