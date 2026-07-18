use a4n_runtime::host_api::config_records::{
    self as config_records_actions, ModelDiscoveryInput, ModelDiscoveryResult,
    UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary,
};

#[tauri::command]
pub async fn list_video_configs(
    state: tauri::State<'_, a4n_runtime::AppState>,
) -> Result<Vec<VideoConfigSummary>, String> {
    config_records_actions::list_video_configs(&state).await
}
tauri_input_command!(
    add_video_config,
    VideoConfigInput,
    VideoConfigSummary,
    config_records_actions::add_video_config,
);
tauri_id_input_command!(
    update_video_config,
    UpdateVideoConfigInput,
    VideoConfigSummary,
    config_records_actions::update_video_config,
);
tauri_id_command!(
    remove_video_config,
    (),
    config_records_actions::remove_video_config,
);
#[tauri::command]
pub async fn activate_video_config(
    state: tauri::State<'_, a4n_runtime::AppState>,
    config_id: i64,
) -> Result<VideoConfigSummary, String> {
    config_records_actions::activate_video_config(&state, config_id).await
}
tauri_id_command!(
    test_video_config,
    bool,
    config_records_actions::test_video_config,
);
tauri_input_command!(
    detect_video_config_models,
    ModelDiscoveryInput,
    ModelDiscoveryResult,
    config_records_actions::detect_video_config_models,
);
