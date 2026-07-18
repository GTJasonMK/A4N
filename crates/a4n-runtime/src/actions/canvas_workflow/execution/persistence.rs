use super::prelude::*;

pub(in crate::actions::canvas_workflow) async fn finish_canvas_run_failed(
    state: &AppState,
    workflow_run_id: i64,
    project_id: i64,
    graph_id: i64,
    mode: &str,
    step_key: String,
    input_json: Value,
    error: String,
) -> Result<CanvasRunSummary, String> {
    record_canvas_node_checkpoint(
        state,
        workflow_run_id,
        step_key,
        WORKFLOW_STATUS_FAILED,
        input_json,
        None,
        Some(error.clone()),
        true,
    )
    .await?;
    let failed = state
        .orchestrator
        .mark_workflow_status(
            workflow_run_id,
            WORKFLOW_STATUS_FAILED,
            true,
            Some(error.as_str()),
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(CanvasRunSummary {
        workflow_run_id: failed.id,
        project_id,
        graph_id,
        mode: mode.to_string(),
        status: failed.status,
        error_message: Some(error),
    })
}

pub(in crate::actions::canvas_workflow) async fn record_canvas_node_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    step_key: String,
    status: &str,
    input_json: Value,
    artifacts_json: Option<Value>,
    error_message: Option<String>,
    completed: bool,
) -> Result<(), String> {
    if status != WORKFLOW_STATUS_COMPLETED {
        state
            .orchestrator
            .mark_workflow_current_step(workflow_run_id, &step_key)
            .await
            .map_err(|e| e.to_string())?;
    }
    state
        .orchestrator
        .upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key: step_key.clone(),
            step_order: 0,
            status: status.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(input_json),
            checkpoint_json: artifacts_json.clone(),
            artifacts_json,
            generation_run_ids_json: None,
            error_message,
            completed,
        })
        .await
        .map_err(|e| e.to_string())?;
    let progress_message = canvas_progress_message(status, &step_key);
    state
        .orchestrator
        .record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id: None,
            current: if status == WORKFLOW_STATUS_RUNNING {
                0
            } else {
                1
            },
            total: 1,
            message: &progress_message,
            meta: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn canvas_progress_message(status: &str, step_key: &str) -> String {
    match status {
        WORKFLOW_STATUS_RUNNING => format!("画布节点运行中: {step_key}"),
        WORKFLOW_STATUS_COMPLETED => format!("画布节点已完成: {step_key}"),
        WORKFLOW_STATUS_FAILED => format!("画布节点失败: {step_key}"),
        other => format!("画布节点状态 {other}: {step_key}"),
    }
}
