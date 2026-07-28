use a4n_core::models::project::novel_project;
use a4n_orchestrator::collaboration::ProjectAccessInfoQuery;
use a4n_orchestrator::project::{ProjectFamilyMetadataQuery, ProjectProgressQuery};

use crate::AppState;
use crate::actions::app_error_message;

use super::super::contracts::{ProjectSummary, contract_progress_summary};
use super::projection::{apply_project_access_info, apply_project_family_metadata, to_summary};

pub async fn project_summary_with_family_for_user(
    state: &AppState,
    user_id: i64,
    project: &novel_project::Model,
) -> Result<ProjectSummary, String> {
    let mut summary = to_summary(project);
    summary.progress = contract_progress_summary(
        state
            .project_read()
            .project_progress_summary(ProjectProgressQuery::from_project(project))
            .await
            .map_err(app_error_message)?,
    );
    let metadata = state
        .project_read()
        .project_family_metadata(ProjectFamilyMetadataQuery::from_project(project))
        .await
        .map_err(app_error_message)?;
    apply_project_family_metadata(&mut summary, &metadata);
    let info = state
        .collaboration()
        .project_access_info(ProjectAccessInfoQuery::from_project(project), user_id)
        .await
        .map_err(app_error_message)?;
    apply_project_access_info(&mut summary, &info);
    Ok(summary)
}
