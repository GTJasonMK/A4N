use crate::AppState;
use a4n_api_contract::config::{ImageConfigInput, ImageConfigSummary, UpdateImageConfigInput};
use a4n_error::AppError;

pub async fn list_image_configs(state: &AppState) -> Result<Vec<ImageConfigSummary>, String> {
    state
        .config_application()
        .list_image()
        .await
        .map_err(|error| error.to_string())
}

pub async fn add_image_config(
    state: &AppState,
    caller_user_id: i64,
    input: ImageConfigInput,
) -> Result<ImageConfigSummary, AppError> {
    state
        .config_application()
        .create_image(caller_user_id, input)
        .await
}

pub async fn update_image_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
    input: UpdateImageConfigInput,
) -> Result<ImageConfigSummary, AppError> {
    state
        .config_application()
        .update_image(caller_user_id, config_id, input)
        .await
}

pub async fn remove_image_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    state
        .config_application()
        .delete_image(caller_user_id, config_id)
        .await
}

pub async fn set_default_image_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    state
        .config_application()
        .set_default_image(caller_user_id, config_id)
        .await
}
