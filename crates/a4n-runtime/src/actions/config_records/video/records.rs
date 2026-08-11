use super::contracts::{
    UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary, video_config_summary,
    video_create_params, video_update_params,
};
use crate::actions::config_records::shared::require_config_admin;
use crate::{AppState, actions::app_error_message};

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
    input: VideoConfigInput,
) -> Result<VideoConfigSummary, String> {
    let params = video_create_params(input)?;
    let config = state
        .orchestrator()
        .create_video_config_validated(params)
        .await
        .map_err(app_error_message)?;
    video_config_summary(&config)
}

pub async fn update_video_config(
    state: &AppState,
    config_id: i64,
    input: UpdateVideoConfigInput,
) -> Result<VideoConfigSummary, String> {
    let existing = state
        .orchestrator()
        .video_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    let params = video_update_params(&existing, input)?;
    let updated = state
        .orchestrator()
        .update_video_config_validated(config_id, params)
        .await
        .map_err(app_error_message)?;
    video_config_summary(&updated)
}

pub async fn remove_video_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .orchestrator()
        .delete_video_config_if_allowed(config_id)
        .await
        .map_err(app_error_message)
}

pub async fn set_default_video_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), String> {
    require_config_admin(state, caller_user_id).await?;
    state
        .orchestrator()
        .set_default_video_config_validated(config_id)
        .await
        .map_err(app_error_message)
}
