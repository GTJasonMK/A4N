use crate::commands::validation::required_positive_u64;
use a4n_core::models::project::novel_project;
use a4n_runtime::AppState;
use a4n_runtime::actions::project_records as project_records_actions;
use a4n_runtime::actions::project_records::{
    CreateDerivedProjectInput, CreateFreeformVideoProjectInput, CreateProjectInput,
    ProjectGeneratedDataResetSummary, ProjectSummary, SourceSnapshotChapterSummary,
    UpdateProjectInput,
};
use a4n_services::project::{AssetDependencyGraph, AssetGraphNodeDetail};
use tauri::State;

#[tauri::command]
pub async fn get_project(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectSummary, String> {
    project_records_actions::get_project_summary(&state, user_id, project_id).await
}

#[tauri::command]
pub async fn get_project_asset_dependency_graph(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<AssetDependencyGraph, String> {
    project_records_actions::get_project_asset_dependency_graph(&state, user_id, project_id).await
}

#[tauri::command]
pub async fn get_project_asset_dependency_graph_node_detail(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    node_id: String,
) -> Result<AssetGraphNodeDetail, String> {
    project_records_actions::get_project_asset_dependency_graph_node_detail(
        &state, user_id, project_id, node_id,
    )
    .await
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, AppState>,
    user_id: i64,
    page: Option<u64>,
    page_size: Option<u64>,
    project_kind: Option<String>,
    root_project_id: Option<i64>,
    access_scope: Option<String>,
) -> Result<serde_json::Value, String> {
    let page = required_positive_u64(page, "page")?;
    let limit = required_positive_u64(page_size, "page_size")?;
    let result = project_records_actions::list_projects(
        &state,
        user_id,
        page,
        limit,
        project_kind.as_deref(),
        root_project_id,
        access_scope.as_deref(),
    )
    .await?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    user_id: i64,
    input: CreateProjectInput,
) -> Result<ProjectSummary, String> {
    project_records_actions::create_project(&state, user_id, input).await
}

#[tauri::command]
pub async fn create_derived_project(
    state: State<'_, AppState>,
    user_id: i64,
    input: CreateDerivedProjectInput,
) -> Result<ProjectSummary, String> {
    project_records_actions::create_derived_project(&state, user_id, input).await
}

#[tauri::command]
pub async fn create_freeform_video_project(
    state: State<'_, AppState>,
    user_id: i64,
    input: CreateFreeformVideoProjectInput,
) -> Result<ProjectSummary, String> {
    project_records_actions::create_freeform_video_project(&state, user_id, input).await
}

#[tauri::command]
pub async fn list_source_snapshot_chapters(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterSummary>, String> {
    project_records_actions::list_source_snapshot_chapters(&state, user_id, project_id).await
}

#[tauri::command]
pub async fn update_project(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: UpdateProjectInput,
) -> Result<ProjectSummary, String> {
    project_records_actions::update_project(&state, user_id, project_id, input).await
}

#[tauri::command]
pub async fn delete_project(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<(), String> {
    project_records_actions::delete_project(&state, user_id, project_id).await
}

#[tauri::command]
pub async fn reset_project_generated_data(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetSummary, String> {
    project_records_actions::reset_project_generated_data(&state, user_id, project_id).await
}

pub(super) async fn project_summary_with_family_for_user(
    state: &State<'_, AppState>,
    user_id: i64,
    project: &novel_project::Model,
) -> Result<ProjectSummary, String> {
    project_records_actions::project_summary_with_family_for_user(state.inner(), user_id, project)
        .await
}
