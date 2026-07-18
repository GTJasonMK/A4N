use crate::AppState;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_orchestrator::generation::generation_checkpoint_fingerprint_sha256;
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use serde_json::{Value, json};

pub(super) async fn start_inspiration_workflow(
    state: &AppState,
    project_id: i64,
    mode: &str,
    variables: Option<&Value>,
    request: &ChatRequest,
    active_task_id: Option<String>,
) -> Result<i64, AppError> {
    let fingerprint = inspiration_workflow_fingerprint(project_id, mode, variables, request)?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: inspiration_business_key(&fingerprint)?,
            workflow_kind: "inspiration_conversation".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("conversation".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(format!(
                "inspiration_conversation.project_{project_id}.turn"
            )),
            active_task_id,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    Ok(workflow_run.id)
}

pub(super) async fn finalize_inspiration_workflow<T>(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<T, AppError>,
) -> Result<(), AppError> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(error) => (WORKFLOW_STATUS_FAILED, true, Some(error.to_string())),
    };
    state
        .orchestrator
        .mark_workflow_status(
            workflow_run_id,
            status,
            can_resume,
            error_message.as_deref(),
        )
        .await?;
    Ok(())
}

fn inspiration_workflow_fingerprint(
    project_id: i64,
    mode: &str,
    variables: Option<&Value>,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "inspiration_conversation",
        "schema_version": "inspiration_conversation_workflow_v1",
        "project_id": project_id,
        "mode": mode,
        "variables": variables.cloned().unwrap_or_else(|| json!({})),
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

fn inspiration_business_key(fingerprint: &Value) -> Result<String, AppError> {
    Ok(format!(
        "inspiration_conversation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}
