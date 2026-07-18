//! 全量配置一键导入导出。
//!
//! 保持 `commands::settings_all::*` 的 Tauri 命令入口稳定，业务逻辑收敛在 runtime。

use a4n_runtime::AppState;
use a4n_runtime::host_api::settings_all::{ImportAllConfigsInput, ImportAllResult};
use tauri::State;

#[tauri::command]
pub async fn export_all_configs_json(state: State<'_, AppState>) -> Result<String, String> {
    a4n_runtime::host_api::settings_all::export_all_configs_to_json(&state)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_all_configs(
    state: State<'_, AppState>,
    input: ImportAllConfigsInput,
) -> Result<ImportAllResult, String> {
    a4n_runtime::host_api::settings_all::import_all_configs(&state, input)
        .await
        .map_err(|e| e.to_string())
}
