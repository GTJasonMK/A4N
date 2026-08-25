use super::mapping::video_config_summary;
use crate::actions::config_records::shared::require_config_admin;
use crate::{AppState, actions::app_error_message};
use a4n_api_contract::config::{UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary};
use a4n_core::error::AppError;

pub async fn list_video_configs(state: &AppState) -> Result<Vec<VideoConfigSummary>, String> {
    let configs = state
        .orchestrator()
        .list_video_config_records()
        .await
        .map_err(app_error_message)?;
    configs.iter().map(video_config_summary).collect()
}

pub async fn add_video_config(
    state: &AppState,
    caller_user_id: i64,
    input: VideoConfigInput,
) -> Result<VideoConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let config = state
        .orchestrator()
        .create_video_config_validated(input)
        .await?;
    video_config_summary(&config).map_err(AppError::InvalidArgument)
}

pub async fn update_video_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
    input: UpdateVideoConfigInput,
) -> Result<VideoConfigSummary, AppError> {
    require_config_admin(state, caller_user_id).await?;
    let updated = state
        .orchestrator()
        .update_video_config_validated(config_id, input)
        .await?;
    video_config_summary(&updated).map_err(AppError::InvalidArgument)
}

pub async fn remove_video_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .delete_video_config_if_allowed(config_id)
        .await
}

pub async fn set_default_video_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .set_default_video_config_validated(config_id)
        .await
}
