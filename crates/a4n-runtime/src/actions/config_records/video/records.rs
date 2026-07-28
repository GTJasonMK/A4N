use super::contracts::{
    UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary, video_config_summary,
    video_create_params, video_update_params,
};
use crate::{AppState, actions::app_error_message};

pub async fn list_video_configs(state: &AppState) -> Result<Vec<VideoConfigSummary>, String> {
    let configs = state
        .settings()
        .list_video_config_records()
        .await
        .map_err(app_error_message)?;
    Ok(configs.iter().map(video_config_summary).collect())
}

pub async fn add_video_config(
    state: &AppState,
    input: VideoConfigInput,
) -> Result<VideoConfigSummary, String> {
    let params = video_create_params(input)?;
    let config = state
        .settings()
        .create_video_config_validated(params)
        .await
        .map_err(app_error_message)?;
    Ok(video_config_summary(&config))
}

pub async fn update_video_config(
    state: &AppState,
    config_id: i64,
    input: UpdateVideoConfigInput,
) -> Result<VideoConfigSummary, String> {
    let existing = state
        .settings()
        .video_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    let params = video_update_params(&existing, input)?;
    let updated = state
        .settings()
        .update_video_config_validated(config_id, params)
        .await
        .map_err(app_error_message)?;
    Ok(video_config_summary(&updated))
}

pub async fn remove_video_config(state: &AppState, config_id: i64) -> Result<(), String> {
    state
        .settings()
        .delete_video_config_if_allowed(config_id)
        .await
        .map_err(app_error_message)
}

pub async fn activate_video_config(
    state: &AppState,
    config_id: i64,
) -> Result<VideoConfigSummary, String> {
    let updated = state
        .settings()
        .set_default_video_config_validated(config_id)
        .await
        .map_err(app_error_message)?;
    Ok(video_config_summary(&updated))
}
