use super::contracts::{
    ImageConfigInput, ImageConfigSummary, UpdateImageConfigInput, image_config_summary,
    image_create_record_input, image_update_record_input,
};
use crate::{AppState, actions::app_error_message};
use a4n_core::config::validate_image_config_model;

pub async fn list_image_configs(state: &AppState) -> Result<Vec<ImageConfigSummary>, String> {
    let configs = state
        .orchestrator
        .list_image_config_records()
        .await
        .map_err(app_error_message)?;
    Ok(configs.iter().map(image_config_summary).collect())
}

pub async fn add_image_config(
    state: &AppState,
    input: ImageConfigInput,
) -> Result<ImageConfigSummary, String> {
    let record_input = image_create_record_input(input)?;
    let config = state
        .orchestrator
        .create_image_config_record(record_input)
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
        .orchestrator
        .image_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    let record_input = image_update_record_input(&existing, input)?;
    let updated = state
        .orchestrator
        .update_image_config_record(existing, record_input)
        .await
        .map_err(app_error_message)?;
    Ok(image_config_summary(&updated))
}

pub async fn remove_image_config(state: &AppState, config_id: i64) -> Result<(), String> {
    let target = state
        .orchestrator
        .image_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    a4n_core::config::ensure_default_config_delete_allowed("图片配置", target.is_default)
        .map_err(|err| err.to_string())?;
    state
        .orchestrator
        .delete_image_config_record(config_id)
        .await
        .map_err(app_error_message)
}

pub async fn activate_image_config(
    state: &AppState,
    config_id: i64,
) -> Result<ImageConfigSummary, String> {
    let target = state
        .orchestrator
        .image_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)?;
    validate_image_config_model(&target)
        .map(drop)
        .map_err(|e| e.to_string())?;
    let updated = state
        .orchestrator
        .set_default_image_config_record(target)
        .await
        .map_err(app_error_message)?;
    Ok(image_config_summary(&updated))
}
