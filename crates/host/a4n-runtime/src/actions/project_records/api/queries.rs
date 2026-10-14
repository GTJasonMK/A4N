use crate::AppState;
use crate::actions::app_error_message;
use a4n_api_contract::collaboration::ProjectAccessScope;
use a4n_api_contract::project::{
    AssetDependencyGraph, AssetGraphNodeDetail, GetSourceSnapshotChapterInput, PaginatedProjects,
    ProjectListInput, ProjectSummary, SourceSnapshotChapterIndex, SourceSnapshotChapterSummary,
};
use a4n_domain::project::ProjectKind;

pub async fn get_project_summary(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectSummary, String> {
    state
        .project_read_application()
        .summary(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn get_project_asset_dependency_graph(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<AssetDependencyGraph, String> {
    state
        .project_asset_graph_application()
        .graph(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn get_project_asset_dependency_graph_node_detail(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    node_id: String,
) -> Result<AssetGraphNodeDetail, String> {
    state
        .project_asset_graph_application()
        .node_detail(user_id, project_id, node_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn list_projects(
    state: &AppState,
    user_id: i64,
    page: u64,
    page_size: u64,
    project_kind: Option<&str>,
    root_project_id: Option<i64>,
    access_scope: Option<&str>,
) -> Result<PaginatedProjects, String> {
    let project_kind = project_kind
        .map(|value| ProjectKind::parse(value).ok_or_else(|| format!("项目族类型无效: {value}")))
        .transpose()?;
    let access_scope = access_scope
        .map(str::parse::<ProjectAccessScope>)
        .transpose()
        .map_err(|error| error.to_string())?;
    state
        .project_read_application()
        .list(
            user_id,
            ProjectListInput {
                page,
                page_size,
                project_kind,
                root_project_id,
                access_scope,
            },
        )
        .await
        .map_err(|error| error.to_string())
}

pub async fn list_source_snapshot_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterSummary>, String> {
    state
        .project_source_snapshot_read_application()
        .chapters(user_id, project_id)
        .await
        .map_err(app_error_message)
}

pub async fn list_source_snapshot_chapter_index(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterIndex>, String> {
    state
        .project_source_snapshot_read_application()
        .chapter_index(user_id, project_id)
        .await
        .map_err(app_error_message)
}

pub async fn get_source_snapshot_chapter(
    state: &AppState,
    user_id: i64,
    input: GetSourceSnapshotChapterInput,
) -> Result<SourceSnapshotChapterSummary, String> {
    state
        .project_source_snapshot_read_application()
        .chapter(user_id, input.project_id, input.source_chapter_id)
        .await
        .map_err(app_error_message)
}
