use super::super::summaries::{
    project_record_by_id, project_summaries_with_family_for_user,
    project_summary_with_family_for_user,
};
use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_api_contract::project::{
    AssetDependencyGraph, AssetGraphNodeDetail, PaginatedProjects, ProjectSummary,
    SourceSnapshotChapterSummary,
};
use a4n_domain::project::ProjectKind;
use a4n_orchestrator::collaboration::{AccessibleProjectListInput, ProjectAccessScope};
use a4n_orchestrator::project::project_list_offset;

pub async fn get_project_summary(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectSummary, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let project = project_record_by_id(state, project_id).await?;
    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn get_project_asset_dependency_graph(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<AssetDependencyGraph, String> {
    state
        .orchestrator()
        .asset_dependency_graph_for_user_validated(user_id, project_id)
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
        .orchestrator()
        .asset_dependency_graph_node_detail_for_user_validated(user_id, project_id, &node_id)
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
    let offset = project_list_offset(page, page_size)?;
    let page_result = state
        .orchestrator()
        .accessible_projects(AccessibleProjectListInput {
            user_id,
            offset,
            limit: page_size,
            project_kind: project_kind
                .map(|value| {
                    ProjectKind::parse(value).ok_or_else(|| format!("项目族类型无效: {value}"))
                })
                .transpose()?,
            root_project_id,
            access_scope: access_scope
                .unwrap_or("all")
                .parse::<ProjectAccessScope>()
                .map_err(|error| error.to_string())?,
        })
        .await
        .map_err(|error| error.to_string())?;
    let projects = page_result
        .items
        .into_iter()
        .map(|project| project.into_model())
        .collect::<Vec<_>>();
    let items = project_summaries_with_family_for_user(state, user_id, &projects).await?;
    Ok(PaginatedProjects {
        items,
        total: page_result.total,
        page,
        page_size,
    })
}

pub async fn list_source_snapshot_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<SourceSnapshotChapterSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    state
        .orchestrator()
        .source_snapshot_chapters(project_id)
        .await
        .map_err(app_error_message)
}
