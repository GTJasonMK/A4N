use super::mapping::image_config_summary;
use crate::actions::config_records::shared::require_config_admin;
use crate::{AppState, actions::app_error_message};
use a4n_api_contract::config::{ImageConfigInput, ImageConfigSummary, UpdateImageConfigInput};
use a4n_core::error::AppError;

pub async fn list_image_configs(state: &AppState) -> Result<Vec<ImageConfigSummary>, String> {
    let configs = state
        .orchestrator()
        .list_image_config_records()
        .await
        .map_err(app_error_message)?;
    configs.iter().map(image_config_summary).collect()
}

pub async fn add_image_config(
    state: &AppState,
    caller_user_id: i64,
    input: ImageConfigInput,
) -> Result<ImageConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let config = state
        .orchestrator()
        .create_image_config_validated(input)
        .await?;
    image_config_summary(&config).map_err(AppError::InvalidArgument)
}

pub async fn update_image_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
    input: UpdateImageConfigInput,
) -> Result<ImageConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let updated = state
        .orchestrator()
        .update_image_config_validated(config_id, input)
        .await?;
    image_config_summary(&updated).map_err(AppError::InvalidArgument)
}

pub async fn remove_image_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .delete_image_config_if_allowed(config_id)
        .await
}

pub async fn set_default_image_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .set_default_image_config_validated(config_id)
        .await
}
