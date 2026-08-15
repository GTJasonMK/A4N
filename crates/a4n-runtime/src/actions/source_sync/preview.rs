use super::{SourceSyncPlanSummary, projections};
use crate::AppState;
use a4n_core::error::AppError;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use a4n_queue::queue::TaskFamilyContext;

pub async fn preview_source_sync(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<SourceSyncPlanSummary, AppError> {
    let family = state
        .orchestrator()
        .source_sync_preview_context_for_user_validated(user_id, derived_project_id)
        .await?;
    let context = TaskFamilyContext::new(
        family.project.id,
        family.root_project.id,
        Some(family.source_snapshot.id),
        "source_sync",
    )?;
    let state_for_task = state.clone();
    let plan = state
        .orchestrator()
        .queue_run_tracked_with_family_context(
            QueueTaskKind::SourceSync,
            TaskPriority::Medium,
            context,
            || async move {
                state_for_task
                    .orchestrator()
                    .preview_source_sync(derived_project_id)
                    .await
            },
        )
        .await?;
    projections::source_sync_plan_summary(plan)
}
