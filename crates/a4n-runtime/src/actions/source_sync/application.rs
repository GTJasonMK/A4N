use super::{ApplySourceSyncInput, SourceSyncRunSummary, orchestrator_decision, projections};
use crate::AppState;
use a4n_core::error::AppError;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use a4n_queue::queue::TaskFamilyContext;

pub async fn apply_source_sync_decisions(
    state: &AppState,
    user_id: i64,
    input: ApplySourceSyncInput,
) -> Result<SourceSyncRunSummary, AppError> {
    let run = state
        .orchestrator()
        .source_sync_run_for_user_validated(user_id, input.sync_run_id)
        .await?;
    let context = TaskFamilyContext::new(
        run.derived_project_id,
        run.root_project_id,
        Some(run.from_snapshot_id),
        "source_sync",
    )?;
    let state_for_task = state.clone();
    let prompt_renderer = state;
    let decisions = input
        .decisions
        .into_iter()
        .map(orchestrator_decision)
        .collect();
    let updated = state
        .orchestrator()
        .queue_run_tracked_with_family_context(
            QueueTaskKind::SourceSync,
            TaskPriority::Medium,
            context,
            || async move {
                state_for_task
                    .orchestrator()
                    .run_source_sync_apply_for_user_validated(
                        user_id,
                        run,
                        decisions,
                        Some(prompt_renderer),
                    )
                    .await
            },
        )
        .await;
    Ok(projections::source_sync_run_summary(&updated?))
}
