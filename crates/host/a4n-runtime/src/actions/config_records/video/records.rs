use crate::AppState;
use a4n_api_contract::config::{UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary};
use a4n_error::AppError;

pub async fn list_video_configs(state: &AppState) -> Result<Vec<VideoConfigSummary>, String> {
    state
        .config_read_application()
        .list_video()
        .await
        .map_err(|error| error.to_string())
}

pub async fn add_video_config(
    state: &AppState,
    caller_user_id: i64,
    input: VideoConfigInput,
) -> Result<VideoConfigSummary, AppError> {
    state
        .config_mutation_application()
        .create_video(caller_user_id, input)
        .await
}

pub async fn update_video_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
    input: UpdateVideoConfigInput,
) -> Result<VideoConfigSummary, AppError> {
    state
        .config_mutation_application()
        .update_video(caller_user_id, config_id, input)
        .await
}

pub async fn remove_video_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    state
        .config_mutation_application()
        .delete_video(caller_user_id, config_id)
        .await
}

pub async fn set_default_video_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    state
        .config_mutation_application()
        .set_default_video(caller_user_id, config_id)
        .await
}
