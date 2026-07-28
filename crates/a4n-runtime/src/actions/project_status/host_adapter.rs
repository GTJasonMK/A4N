//! Host 适配：读库组装回退预览（纯规则在 orchestrator）。

use super::RollbackImpactSummary;
use crate::{AppState, actions::app_error_message};
use a4n_core::{models::project::novel_project, state::status::ProjectStatus};
use a4n_orchestrator::project::rollback_impact_query_for_transition;

pub(super) async fn project_record_by_id(
    state: &AppState,
    project_id: i64,
) -> Result<novel_project::Model, String> {
    state
        .project_read()
        .project_by_id(project_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
}

pub(super) async fn build_rollback_impacts(
    state: &AppState,
    project_id: i64,
    from: ProjectStatus,
    to: ProjectStatus,
) -> Result<Vec<RollbackImpactSummary>, String> {
    state
        .project_read()
        .project_rollback_impacts(rollback_impact_query_for_transition(project_id, from, to))
        .await
        .map_err(app_error_message)
}
