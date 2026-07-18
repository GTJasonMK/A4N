//! 导入命令
//!
//! 导入审核队列与分析入口；业务经 runtime actions / orchestrator 执行。

use a4n_runtime::AppState;
pub mod analysis;
pub mod reverse;
pub mod review;

use tauri::State;

use a4n_core::models::project::import_review_queue;
use a4n_runtime::host_api::import_analysis::StartImportAnalysisInput;
use a4n_runtime::host_api::import_file::{ImportFileResult, ImportUploadedFileInput};
use a4n_runtime::host_api::import_review as import_review_actions;
use a4n_runtime::host_api::import_review::{
    ImportAnalysisStatusSummary, ImportReviewSummary, UpdateImportStatusInput,
};

pub(super) fn to_summary(m: &import_review_queue::Model) -> Result<ImportReviewSummary, String> {
    a4n_runtime::host_api::import_review::import_review_summary(m).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_import_queue(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ImportReviewSummary>, String> {
    import_review_actions::list_import_queue(&state, user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_import_status(
    state: State<'_, AppState>,
    user_id: i64,
    review_id: i64,
    input: UpdateImportStatusInput,
) -> Result<ImportReviewSummary, String> {
    import_review_actions::update_import_status(&state, user_id, review_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_import_review(
    state: State<'_, AppState>,
    user_id: i64,
    review_id: i64,
) -> Result<(), String> {
    import_review_actions::delete_import_review(&state, user_id, review_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_uploaded_file(
    _state: State<'_, AppState>,
    input: ImportUploadedFileInput,
) -> Result<ImportFileResult, String> {
    a4n_runtime::host_api::import_file::import_uploaded_file(input)
}

#[tauri::command]
pub async fn start_import_analysis(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: StartImportAnalysisInput,
) -> Result<Vec<ImportReviewSummary>, String> {
    let reviews = a4n_runtime::host_api::import_analysis::start_import_analysis(
        state.inner(),
        user_id,
        project_id,
        input,
    )
    .await?;
    reviews.iter().map(to_summary).collect()
}

#[tauri::command]
pub async fn get_import_analysis_status(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ImportAnalysisStatusSummary>, String> {
    import_review_actions::get_import_analysis_status(&state, user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}
