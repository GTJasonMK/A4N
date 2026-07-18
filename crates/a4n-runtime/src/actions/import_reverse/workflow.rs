use super::contracts::ReverseEngineerInput;
use super::target::ReverseEngineeringTarget;
use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use serde_json::{Value, json};
use std::collections::HashMap;

pub(super) async fn start_reverse_workflow_run(
    state: &AppState,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    apply: bool,
    content_sample: &str,
    variables: &HashMap<String, Value>,
) -> Result<i64, AppError> {
    let mode = if apply { "apply" } else { "reverse" };
    let run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "import_reverse_engineer:project:{}:{}:{mode}",
                input.project_id,
                target.prompt_name()
            ),
            workflow_kind: "import_reverse_engineer".to_string(),
            project_id: Some(input.project_id),
            root_project_id: Some(input.project_id),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(input.project_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(json!({
                "schema_version": 1,
                "workflow": "import_reverse_engineer",
                "target": target.prompt_name(),
                "mode": mode,
                "project_id": input.project_id,
                "content_sample_sha256": source_content_sha256(content_sample),
                "auto_context": input.auto_context,
                "context_limit": input.context_limit,
                "variables": variables
            })),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(target.prompt_name().to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    Ok(run.id)
}

pub(super) async fn complete_reverse_workflow_run(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<(), AppError> {
    state
        .orchestrator
        .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await?;
    Ok(())
}

pub(super) async fn fail_reverse_workflow_run(
    state: &AppState,
    workflow_run_id: i64,
    error: &AppError,
) -> Result<(), AppError> {
    state
        .orchestrator
        .mark_workflow_status(
            workflow_run_id,
            WORKFLOW_STATUS_FAILED,
            true,
            Some(&error.to_string()),
        )
        .await?;
    Ok(())
}
