use super::super::shared::{
    ModelDiscoveryInput, model_discovery_api_key, required_existing_config_id,
};
use crate::{AppState, actions::app_error_message};
use a4n_model_discovery::ModelDiscoveryResult;

pub async fn detect_video_config_models(
    state: &AppState,
    input: ModelDiscoveryInput,
) -> Result<ModelDiscoveryResult, String> {
    let api_key = model_discovery_api_key(input.api_key_encrypted.as_deref(), || async {
        if !input.use_saved_api_key {
            return Ok(None);
        }
        let config_id = required_existing_config_id(input.config_id, "视频")?;
        let config = state
            .settings()
            .video_config_record_by_id(config_id)
            .await
            .map_err(app_error_message)?;
        Ok(config.api_key_encrypted)
    })
    .await?;
    crate::sidecar_clients::discover_video_models(
        state.runtime_profile(),
        state.capabilities(),
        &input.provider,
        &input.base_url,
        api_key.as_deref(),
    )
    .await
}
