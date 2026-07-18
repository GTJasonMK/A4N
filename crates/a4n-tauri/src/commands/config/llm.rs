use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_runtime::host_api::config_records::{
    self as config_records_actions, CreateLlmInput, LlmConfigSummary, ModelDiscoveryInput,
    ModelDiscoveryResult, ModelRequestPreview, UpdateLlmInput,
};

#[tauri::command]
pub async fn list_llm_config(
    state: tauri::State<'_, a4n_runtime::AppState>,
) -> Result<Vec<LlmConfigSummary>, String> {
    config_records_actions::list_llm_config(&state).await
}
tauri_input_command!(
    create_llm_config,
    CreateLlmInput,
    LlmConfigSummary,
    config_records_actions::create_llm_config,
);
tauri_id_input_command!(
    update_llm_config,
    UpdateLlmInput,
    LlmConfigSummary,
    config_records_actions::update_llm_config,
);
#[tauri::command]
pub async fn set_default_llm_config(
    state: tauri::State<'_, a4n_runtime::AppState>,
    config_id: i64,
) -> Result<(), String> {
    config_records_actions::set_default_llm_config(&state, config_id).await
}
tauri_id_command!(
    delete_llm_config,
    (),
    config_records_actions::delete_llm_config,
);
tauri_state_command!(
    get_llm_max_tokens_config,
    LlmMaxTokensConfig,
    config_records_actions::get_llm_max_tokens_config,
);
tauri_input_command!(
    update_llm_max_tokens_config,
    LlmMaxTokensConfig,
    LlmMaxTokensConfig,
    config_records_actions::update_llm_max_tokens_config,
);
tauri_id_command!(
    test_llm_config,
    bool,
    config_records_actions::test_llm_config,
);
tauri_id_command!(
    preview_test_llm_config_request,
    ModelRequestPreview,
    config_records_actions::preview_test_llm_config_request,
);
tauri_input_command!(
    detect_llm_config_models,
    ModelDiscoveryInput,
    ModelDiscoveryResult,
    config_records_actions::detect_llm_config_models,
);
