use a4n_runtime::host_api::config_records::{self as config_records_actions, UsageGuideDocument};

#[tauri::command]
pub async fn get_usage_guide(
    state: tauri::State<'_, a4n_runtime::AppState>,
) -> Result<UsageGuideDocument, String> {
    config_records_actions::get_usage_guide(&state)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_usage_guide(
    state: tauri::State<'_, a4n_runtime::AppState>,
    caller_user_id: i64,
    input: UsageGuideDocument,
) -> Result<UsageGuideDocument, String> {
    config_records_actions::update_usage_guide(&state, caller_user_id, input)
        .await
        .map_err(|error| error.to_string())
}
