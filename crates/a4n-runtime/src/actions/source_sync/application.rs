use super::{ApplySourceSyncInput, SourceSyncRunSummary, access, projections};
use crate::actions::workflow_status_write_app_error;
use crate::{AppState, prompt_renderer::RuntimePromptRenderer};
use a4n_core::error::AppError;
use a4n_orchestrator::source_sync::{
    GalgameSourceSyncExecutionContext, SourceSyncExecutionContext,
};
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_queue::queue::TaskFamilyContext;
use serde_json::json;

pub async fn apply_source_sync_decisions(
    state: &AppState,
    user_id: i64,
    input: ApplySourceSyncInput,
) -> Result<SourceSyncRunSummary, AppError> {
    let run = access::verify_run_access(state, user_id, input.sync_run_id).await?;
    let context = TaskFamilyContext::new(
        run.derived_project_id,
        run.root_project_id,
        Some(run.from_snapshot_id),
        "source_sync",
    )?;
    let state_for_task = state.clone();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let sync_run_id = input.sync_run_id;
    let decisions = input.decisions;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!("source_sync_apply:run:{sync_run_id}"),
            workflow_kind: "source_sync_apply".to_string(),
            project_id: Some(run.derived_project_id),
            root_project_id: Some(run.root_project_id),
            source_snapshot_id: Some(run.to_snapshot_id),
            scope_table: Some("source_sync_run".to_string()),
            scope_id: Some(sync_run_id),
            chapter_id: None,
            mode: Some("apply".to_string()),
            input_fingerprint_json: Some(json!({
                "schema_version": 1,
                "workflow": "source_sync_apply",
                "sync_run_id": sync_run_id,
                "derived_project_id": run.derived_project_id,
                "root_project_id": run.root_project_id,
                "from_snapshot_id": run.from_snapshot_id,
                "to_snapshot_id": run.to_snapshot_id,
                "decisions": decisions,
            })),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("apply_decisions".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let workflow_run_id = workflow_run.id;
    let updated = state
        .task_queue
        .run_tracked_with_family_context(
            QueueTaskType::SourceSync,
            TaskPriority::Medium,
            context,
            || async move {
                let execution_context = SourceSyncExecutionContext {
                    galgame: Some(GalgameSourceSyncExecutionContext {
                        prompt_renderer: &prompt_renderer,
                        workflow_run_id,
                    }),
                };
                state_for_task
                    .apply_source_sync_decisions_with_context(
                        sync_run_id,
                        decisions,
                        execution_context,
                    )
                    .await
            },
        )
        .await;
    match updated {
        Ok(updated) => {
            state
                .orchestrator
                .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await?;
            Ok(projections::source_sync_run_summary(&updated))
        }
        Err(error) => {
            if let Err(status_error) = state
                .orchestrator
                .mark_workflow_status(
                    workflow_run_id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(&error.to_string()),
                )
                .await
            {
                return Err(workflow_status_write_app_error(&error, &status_error));
            }
            Err(error)
        }
    }
}
