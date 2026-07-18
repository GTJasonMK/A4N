use super::basic::project_summary_with_family_for_user;
use a4n_runtime::AppState;
use a4n_runtime::host_api::project_records::ProjectSummary;
use a4n_runtime::host_api::project_transfer::{
    ExportDerivedProjectDataInput, ExportProjectFamilyDataInput, ExportProjectPackageData,
    ExportVideoStudioProjectDataInput, ImportProjectFamilyUploadInput,
    ImportProjectFullUploadInput, ImportVideoStudioProjectUploadInput,
};
use tauri::State;

#[tauri::command]
pub async fn import_project_full_upload(
    state: State<'_, AppState>,
    user_id: i64,
    input: ImportProjectFullUploadInput,
) -> Result<ProjectSummary, String> {
    let project = a4n_runtime::host_api::project_transfer::import_project_full_upload(
        state.inner(),
        user_id,
        input,
    )
    .await?;
    project_summary_with_family_for_user(&state, user_id, &project).await
}

#[tauri::command]
pub async fn export_project_family_data(
    state: State<'_, AppState>,
    user_id: i64,
    input: ExportProjectFamilyDataInput,
) -> Result<ExportProjectPackageData, String> {
    a4n_runtime::host_api::project_transfer::export_project_family_data(
        state.inner(),
        user_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn export_derived_project_data(
    state: State<'_, AppState>,
    user_id: i64,
    input: ExportDerivedProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    a4n_runtime::host_api::project_transfer::export_derived_project_data(
        state.inner(),
        user_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn import_project_family_upload(
    state: State<'_, AppState>,
    user_id: i64,
    input: ImportProjectFamilyUploadInput,
) -> Result<Vec<ProjectSummary>, String> {
    let projects = a4n_runtime::host_api::project_transfer::import_project_family_upload(
        state.inner(),
        user_id,
        input,
    )
    .await?;
    let mut summaries = Vec::with_capacity(projects.len());
    for project in projects {
        summaries.push(project_summary_with_family_for_user(&state, user_id, &project).await?);
    }
    Ok(summaries)
}

#[tauri::command]
pub async fn export_video_studio_project_data(
    state: State<'_, AppState>,
    user_id: i64,
    input: ExportVideoStudioProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    a4n_runtime::host_api::project_transfer::export_video_studio_project_data(
        state.inner(),
        user_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn import_video_studio_project_upload(
    state: State<'_, AppState>,
    user_id: i64,
    input: ImportVideoStudioProjectUploadInput,
) -> Result<ProjectSummary, String> {
    let project = a4n_runtime::host_api::project_transfer::import_video_studio_project_upload(
        state.inner(),
        user_id,
        input,
    )
    .await?;
    project_summary_with_family_for_user(&state, user_id, &project).await
}
