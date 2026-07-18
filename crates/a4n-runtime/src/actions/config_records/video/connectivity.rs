use super::super::shared::validate_runtime_snapshot_config_id;
use crate::{AppState, actions::app_error_message};
use a4n_core::models::ai::video_gen_config;
use a4n_orchestrator::video::pick_provider_for_config;

pub async fn test_video_config(state: &AppState, config_id: i64) -> Result<bool, String> {
    let config = resolve_video_config_for_connectivity_test(state, config_id).await?;
    if state.runtime_profile.uses_sidecars() {
        crate::sidecar_clients::probe_video_config(
            state.runtime_profile,
            state.capabilities(),
            &config,
        )
        .await?;
        return Ok(true);
    }
    pick_provider_for_config(&config)?;
    Ok(true)
}

async fn resolve_video_config_for_connectivity_test(
    state: &AppState,
    config_id: i64,
) -> Result<video_gen_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_video_model() {
        validate_runtime_snapshot_config_id("video", config.id, config_id)?;
        return Ok(config);
    }
    state
        .orchestrator
        .video_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)
}
