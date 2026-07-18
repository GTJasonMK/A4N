use super::contracts::{
    UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary, video_config_summary,
    video_create_record_input, video_update_record_input,
};
use crate::{AppState, actions::app_error_message};
use a4n_core::error::AppError;
use a4n_orchestrator::video::validate_video_config_model;

pub async fn list_video_configs(state: &AppState) -> Result<Vec<VideoConfigSummary>, String> {
    let configs = state
        .orchestrator
        .list_video_config_records()
        .await
        .map_err(app_error_message)?;
    Ok(configs.iter().map(video_config_summary).collect())
}

pub async fn add_video_config(
    state: &AppState,
    input: VideoConfigInput,
) -> Result<VideoConfigSummary, String> {
    let record_input = video_create_record_input(input)?;
    let config = state
        .orchestrator
        .create_video_config_record(record_input)
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
        .orchestrator
        .video_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    let record_input = video_update_record_input(&existing, input)?;
    let updated = state
        .orchestrator
        .update_video_config_record(existing, record_input)
        .await
        .map_err(app_error_message)?;
    Ok(video_config_summary(&updated))
}

pub async fn remove_video_config(state: &AppState, config_id: i64) -> Result<(), String> {
    let target = state
        .orchestrator
        .video_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    a4n_core::config::ensure_default_config_delete_allowed("视频配置", target.is_default)
        .map_err(|err| err.to_string())?;
    let active_video_ids = state
        .orchestrator
        .active_generated_video_ids_for_config(config_id)
        .await
        .map_err(app_error_message)?;
    if !active_video_ids.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "视频配置 #{config_id} 仍有 {} 个进行中的视频任务，不能删除",
            active_video_ids.len()
        ))
        .to_string());
    }
    state
        .orchestrator
        .delete_video_config_record(config_id)
        .await
        .map_err(app_error_message)
}

pub async fn activate_video_config(
    state: &AppState,
    config_id: i64,
) -> Result<VideoConfigSummary, String> {
    let target = state
        .orchestrator
        .video_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    validate_video_config_model(&target)
        .map_err(|err| AppError::InvalidArgument(err).to_string())?;
    let updated = state
        .orchestrator
        .set_default_video_config_record(target)
        .await
        .map_err(app_error_message)?;
    Ok(video_config_summary(&updated))
}
