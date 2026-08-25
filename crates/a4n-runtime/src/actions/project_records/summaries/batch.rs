use a4n_core::models::project::novel_project;
use a4n_orchestrator::collaboration::ProjectAccessInfoQuery;
use a4n_orchestrator::project::ProjectFamilyMetadataQuery;

use crate::AppState;
use crate::actions::app_error_message;

use super::{project_progress_query, projection::to_summary};
use a4n_api_contract::project::ProjectSummary;

pub async fn project_summaries_with_family_for_user(
    state: &AppState,
    user_id: i64,
    projects: &[novel_project::Model],
) -> Result<Vec<ProjectSummary>, String> {
    let family_queries: Vec<_> = projects
        .iter()
        .map(ProjectFamilyMetadataQuery::from_project)
        .collect();
    let access_queries: Vec<_> = projects
        .iter()
        .map(ProjectAccessInfoQuery::from_project)
        .collect();
    let progress_queries: Vec<_> = projects
        .iter()
        .map(project_progress_query)
        .collect::<Result<_, _>>()?;
    let orchestrator = state.orchestrator();
    let (family_metadata_by_project_id, access_infos, progress_by_project_id) = tokio::try_join!(
        orchestrator.project_family_metadata_batch(&family_queries),
        orchestrator.project_access_info_batch(&access_queries, user_id),
        orchestrator.project_progress_summaries(&progress_queries),
    )
    .map_err(app_error_message)?;

    let mut summaries = Vec::with_capacity(projects.len());
    for (project, query) in projects.iter().zip(&progress_queries) {
        let progress = progress_by_project_id
            .get(&project.id)
            .cloned()
            .ok_or_else(|| format!("项目缺少进度摘要: {}", project.id))?;
        let family_metadata = family_metadata_by_project_id
            .get(&project.id)
            .ok_or_else(|| format!("项目缺少族元数据: {}", project.id))?;
        let info = access_infos
            .get(&project.id)
            .ok_or_else(|| format!("项目缺少协作访问信息: {}", project.id))?;
        summaries.push(to_summary(
            project,
            family_metadata,
            info,
            query.project_kind,
            query.source_mode,
            query.status,
            progress,
        ));
    }
    Ok(summaries)
}
