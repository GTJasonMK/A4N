use super::contracts::{
    ImageConfigInput, ImageConfigSummary, UpdateImageConfigInput, image_config_summary,
    image_create_params, image_update_params,
};
use crate::{AppState, actions::app_error_message};

pub async fn list_image_configs(state: &AppState) -> Result<Vec<ImageConfigSummary>, String> {
    let configs = state
        .settings()
        .list_image_config_records()
        .await
        .map_err(app_error_message)?;
    Ok(configs.iter().map(image_config_summary).collect())
}

pub async fn add_image_config(
    state: &AppState,
    input: ImageConfigInput,
) -> Result<ImageConfigSummary, String> {
    let params = image_create_params(input)?;
    let config = state
        .settings()
        .create_image_config_validated(params)
        .await
        .map_err(app_error_message)?;
    Ok(image_config_summary(&config))
}

pub async fn update_image_config(
    state: &AppState,
    config_id: i64,
    input: UpdateImageConfigInput,
) -> Result<ImageConfigSummary, String> {
    let existing = state
        .settings()
        .image_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    let params = image_update_params(&existing, input)?;
    let updated = state
        .settings()
        .update_image_config_validated(config_id, params)
        .await
        .map_err(app_error_message)?;
    Ok(image_config_summary(&updated))
}

pub async fn remove_image_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .settings()
        .delete_image_config_if_allowed(config_id)
        .await
        .map_err(app_error_message)
}

pub async fn activate_image_config(
    state: &AppState,
    config_id: i64,
) -> Result<ImageConfigSummary, String> {
    let updated = state
        .settings()
        .set_default_image_config_validated(config_id)
        .await
        .map_err(app_error_message)?;
    Ok(image_config_summary(&updated))
}
