use a4n_runtime::host_api::config_records::{
    self as config_records_actions, ImageConfigInput, ImageConfigSummary, ModelDiscoveryInput,
    ModelDiscoveryResult, ModelRequestPreview, UpdateImageConfigInput,
};

#[tauri::command]
pub async fn list_image_configs(
    state: tauri::State<'_, a4n_runtime::AppState>,
) -> Result<Vec<ImageConfigSummary>, String> {
    config_records_actions::list_image_configs(&state).await
}
tauri_input_command!(
    add_image_config,
    ImageConfigInput,
    ImageConfigSummary,
    config_records_actions::add_image_config,
);
tauri_id_input_command!(
    update_image_config,
    UpdateImageConfigInput,
    ImageConfigSummary,
    config_records_actions::update_image_config,
);
tauri_id_command!(
    remove_image_config,
    (),
    config_records_actions::remove_image_config,
);
tauri_id_command!(
    activate_image_config,
    (),
    config_records_actions::activate_image_config,
);
tauri_id_command!(
    test_image_config,
    (),
    config_records_actions::test_image_config,
);
tauri_id_command!(
    preview_test_image_config_request,
    ModelRequestPreview,
    config_records_actions::preview_test_image_config_request,
);
tauri_input_command!(
    detect_image_config_models,
    ModelDiscoveryInput,
    ModelDiscoveryResult,
    config_records_actions::detect_image_config_models,
);
