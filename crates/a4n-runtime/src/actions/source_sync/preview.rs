use super::{SourceSyncPlanSummary, access, projections};
use crate::AppState;
use a4n_core::error::AppError;
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_queue::queue::TaskFamilyContext;

pub async fn preview_source_sync(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<SourceSyncPlanSummary, AppError> {
    let family =
        access::verify_derived_project_family_editor_context(state, user_id, derived_project_id)
            .await?;
    let context = TaskFamilyContext::new(
        family.project.id,
        family.root_project.id,
        Some(family.source_snapshot.id),
        "source_sync",
    )?;
    let state_for_task = state.clone();
    let plan = state
        .task_queue
        .run_tracked_with_family_context(
            QueueTaskType::SourceSync,
            TaskPriority::Medium,
            context,
            || async move { state_for_task.preview_source_sync(derived_project_id).await },
        )
        .await?;
    access::verify_plan_access(state, user_id, &plan).await?;
    Ok(projections::source_sync_plan_summary(plan))
}
