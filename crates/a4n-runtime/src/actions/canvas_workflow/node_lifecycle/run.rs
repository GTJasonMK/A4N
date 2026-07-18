use super::super::execution::record_canvas_node_checkpoint;
use super::super::persistence::mark_canvas_generation_spec_status;
use super::super::{CanvasRunSummary, WORKFLOW_KIND_VIDEO_CANVAS_GRAPH};
use crate::state::AppState;
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_RUNNING,
};
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub(in crate::actions::canvas_workflow) struct CanvasRunHandle {
    pub(in crate::actions::canvas_workflow) workflow_run_id: i64,
    pub(in crate::actions::canvas_workflow) finalize_on_success: bool,
}

pub(in crate::actions::canvas_workflow) async fn start_canvas_node_run(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    mode: &str,
    step_key: &str,
    input_json: Value,
    shared_run: Option<CanvasRunHandle>,
) -> Result<CanvasRunHandle, String> {
    if let Some(run) = shared_run {
        record_canvas_node_checkpoint(
            state,
            run.workflow_run_id,
            step_key.to_string(),
            WORKFLOW_STATUS_RUNNING,
            input_json,
            None,
            None,
            false,
        )
        .await?;
        return Ok(run);
    }
    let run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!("video_canvas_graph:{graph_id}:{mode}:{step_key}"),
            workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
            project_id: Some(project_id),
            root_project_id: Some(project_id),
            source_snapshot_id: None,
            scope_table: Some("canvas_graph".to_string()),
            scope_id: Some(graph_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(input_json),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(step_key.to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(CanvasRunHandle {
        workflow_run_id: run.id,
        finalize_on_success: true,
    })
}

pub(in crate::actions::canvas_workflow) async fn finish_canvas_node_success(
    state: &AppState,
    run: CanvasRunHandle,
    project_id: i64,
    graph_id: i64,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    if run.finalize_on_success {
        let completed = state
            .orchestrator
            .mark_workflow_status(run.workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(CanvasRunSummary {
            workflow_run_id: completed.id,
            project_id,
            graph_id,
            mode: mode.to_string(),
            status: completed.status,
            error_message: None,
        });
    }
    Ok(CanvasRunSummary {
        workflow_run_id: run.workflow_run_id,
        project_id,
        graph_id,
        mode: mode.to_string(),
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        error_message: None,
    })
}

pub(in crate::actions::canvas_workflow) async fn finish_canvas_node_cancelled(
    state: &AppState,
    run: CanvasRunHandle,
    project_id: i64,
    graph_id: i64,
    mode: &str,
    step_key: String,
    input_json: Value,
    generation_spec_id: Option<i64>,
) -> Result<CanvasRunSummary, String> {
    if let Some(spec_id) = generation_spec_id {
        mark_canvas_generation_spec_status(state, spec_id, WORKFLOW_STATUS_CANCELLED).await?;
    }
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_CANCELLED,
        input_json,
        None,
        Some("用户取消画布运行".to_string()),
        true,
    )
    .await?;
    Ok(CanvasRunSummary {
        workflow_run_id: run.workflow_run_id,
        project_id,
        graph_id,
        mode: mode.to_string(),
        status: WORKFLOW_STATUS_CANCELLED.to_string(),
        error_message: Some("用户取消画布运行".to_string()),
    })
}
