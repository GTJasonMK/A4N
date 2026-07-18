use super::core::{log_dir, reload_filter};
use super::{LogReadResult, ReadLogsInput, SetLogLevelInput};
use a4n_runtime::host_api::logging as logging_actions;
use serde::Deserialize;
use tauri::State;

#[tauri::command]
pub async fn get_log_dir(
    state: State<'_, a4n_runtime::AppState>,
    caller_user_id: i64,
) -> Result<String, String> {
    let log_dir = log_dir().ok_or_else(|| "日志系统尚未初始化".to_string())?;
    logging_actions::log_dir_path_for_admin(&state, caller_user_id, log_dir)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn read_recent_logs(
    state: State<'_, a4n_runtime::AppState>,
    caller_user_id: i64,
    input: ReadLogsInput,
) -> Result<LogReadResult, String> {
    let dir = log_dir().ok_or_else(|| "日志系统尚未初始化".to_string())?;
    logging_actions::read_recent_logs_for_admin(&state, caller_user_id, dir, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn set_log_level(
    state: State<'_, a4n_runtime::AppState>,
    caller_user_id: i64,
    input: SetLogLevelInput,
) -> Result<(), String> {
    logging_actions::ensure_log_diagnostics_admin(&state, caller_user_id)
        .await
        .map_err(|err| err.to_string())?;
    let new_filter =
        logging_actions::build_log_filter(&input.filter).map_err(|err| err.to_string())?;
    reload_filter(new_filter)?;
    tracing::info!(target: "a4n::logging", filter = %input.filter, "日志级别已更新");
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct FrontendLogInput {
    pub level: String,
    pub target: Option<String>,
    pub message: String,
    pub fields: Option<serde_json::Value>,
}

#[tauri::command]
pub fn log_from_frontend(input: FrontendLogInput) -> Result<(), String> {
    let target = input.target.unwrap_or_else(|| "frontend".to_string());
    let fields_str = input
        .fields
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();

    match input.level.to_lowercase().as_str() {
        "trace" => {
            tracing::trace!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        "debug" => {
            tracing::debug!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        "info" => {
            tracing::info!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        "warn" => {
            tracing::warn!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        "error" => {
            tracing::error!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        other => return Err(format!("未知日志级别: {other}")),
    }
    Ok(())
}
