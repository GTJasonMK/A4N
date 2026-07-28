use a4n_core::models::project::novel_project;
use a4n_orchestrator::collaboration::ProjectAccessInfoQuery;
use a4n_orchestrator::project::{ProjectFamilyMetadataQuery, ProjectProgressQuery};

use crate::AppState;
use crate::actions::app_error_message;

use super::super::contracts::{ProjectSummary, contract_progress_summary};
use super::projection::{apply_project_access_info, apply_project_family_metadata, to_summary};

pub async fn project_summaries_with_family_for_user(
    state: &AppState,
    user_id: i64,
    projects: &[novel_project::Model],
) -> Result<Vec<ProjectSummary>, String> {
    let family_queries: Vec<_> = projects
        .iter()
        .map(ProjectFamilyMetadataQuery::from_project)
        .collect();
    let family_metadata_by_project_id = state
        .project_read()
        .project_family_metadata_batch(&family_queries)
        .await
        .map_err(app_error_message)?;
    let access_queries: Vec<_> = projects
        .iter()
        .map(ProjectAccessInfoQuery::from_project)
        .collect();
    let access_infos = state
        .collaboration()
        .project_access_info_batch(&access_queries, user_id)
        .await
        .map_err(app_error_message)?;
    let progress_queries: Vec<_> = projects
        .iter()
        .map(ProjectProgressQuery::from_project)
        .collect();
    let progress_by_project_id = state
        .project_read()
        .project_progress_summaries(&progress_queries)
        .await
        .map_err(app_error_message)?;

    let mut summaries = Vec::with_capacity(projects.len());
    for project in projects {
        let mut summary = to_summary(project);
        summary.progress = contract_progress_summary(
            progress_by_project_id
                .get(&project.id)
                .cloned()
                .ok_or_else(|| format!("项目缺少进度摘要: {}", project.id))?,
        );
        let family_metadata = family_metadata_by_project_id
            .get(&project.id)
            .ok_or_else(|| format!("项目缺少族元数据: {}", project.id))?;
        apply_project_family_metadata(&mut summary, family_metadata);
        let info = access_infos
            .get(&project.id)
            .ok_or_else(|| format!("项目缺少协作访问信息: {}", project.id))?;
        apply_project_access_info(&mut summary, info);
        summaries.push(summary);
    }
    Ok(summaries)
}
