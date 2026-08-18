use a4n_capabilities::A4nCapabilities;
use a4n_core::models::ai::image_gen_config;
use a4n_image::{ImageGenerationRequest, ImageGenerationResult};

use crate::state::RuntimeProfile;

use super::super::super::missing_sidecar_capability_message;
use super::super::contracts::image_provider_error_with_config;
use super::sidecar::generate_image_via_sidecar;

pub(crate) async fn generate_image(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    config: &image_gen_config::Model,
    request: ImageGenerationRequest,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    progress: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<ImageGenerationResult, String> {
    if profile.uses_sidecars() {
        let client = capabilities
            .image
            .clone()
            .ok_or_else(|| missing_sidecar_capability_message("image"))?;
        return generate_image_via_sidecar(
            client,
            config,
            request,
            project_id,
            root_project_id,
            workflow_run_id,
            progress,
        )
        .await;
    }

    let provider =
        a4n_orchestrator::config::configured_image_provider(config).map_err(|e| e.to_string())?;
    provider
        .generate(request)
        .await
        .map_err(|e| image_provider_error_with_config(config, e.to_string()))
}
