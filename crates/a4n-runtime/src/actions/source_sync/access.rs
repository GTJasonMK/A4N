use crate::{
    AppState,
    actions::access::{verify_derived_project_family_context, verify_project_editor},
};
use a4n_core::{error::AppError, models::source_sync::source_sync_run};
use a4n_orchestrator::{project::ProjectFamilyContext, source_sync::SourceSyncPlan};

pub(super) async fn verify_derived_project_family_editor_context(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<ProjectFamilyContext, AppError> {
    let family = verify_derived_project_family_context(state, user_id, derived_project_id).await?;
    verify_project_editor(state, user_id, family.project.id).await?;
    verify_project_editor(state, user_id, family.root_project.id).await?;
    Ok(family)
}

pub(super) async fn verify_plan_access(
    state: &AppState,
    user_id: i64,
    plan: &SourceSyncPlan,
) -> Result<(), AppError> {
    verify_project_editor(state, user_id, plan.run.derived_project_id).await?;
    verify_project_editor(state, user_id, plan.run.root_project_id).await
}

pub(super) async fn verify_run_access(
    state: &AppState,
    user_id: i64,
    sync_run_id: i64,
) -> Result<source_sync_run::Model, AppError> {
    let run = state
        .orchestrator
        .source_sync_run_by_id(sync_run_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("源同步运行不存在: {sync_run_id}")))?;
    verify_project_editor(state, user_id, run.derived_project_id).await?;
    verify_project_editor(state, user_id, run.root_project_id).await?;
    Ok(run)
}
