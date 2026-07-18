//! 项目级导入分析命令。
//!
//! 保持 `commands::import::analysis::*` 的 Tauri 命令入口稳定，具体职责下沉到子模块。

use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::import_analysis::ImportProjectAnalysisSummary;
use serde_json::Value;
use tauri::State;

/// 获取项目级导入分析进度。
#[tauri::command]
pub async fn get_project_import_analysis_progress(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Option<Value>, String> {
    a4n_runtime::host_api::import_analysis::get_project_import_analysis_progress(
        state.inner(),
        user_id,
        project_id,
    )
    .await
}

/// 对已导入章节执行项目级导入分析，补齐结构化分析、章节大纲、基础蓝图和分部大纲。
#[tauri::command]
pub async fn run_import_project_analysis(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<ImportProjectAnalysisSummary, String> {
    a4n_runtime::host_api::import_analysis::run_import_project_analysis(
        state.inner(),
        user_id,
        project_id,
    )
    .await
}

#[tauri::command]
pub async fn preview_import_project_analysis_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<ModelRequestPreview, String> {
    a4n_runtime::host_api::import_analysis::preview_import_project_analysis_request(
        state.inner(),
        user_id,
        project_id,
    )
    .await
}

/// 取消导入分析：删除项目中所有待处理（pending）的审核记录。
#[tauri::command]
pub async fn cancel_import_analysis(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<u64, String> {
    a4n_runtime::host_api::import_analysis::cancel_import_analysis(
        state.inner(),
        user_id,
        project_id,
    )
    .await
}
