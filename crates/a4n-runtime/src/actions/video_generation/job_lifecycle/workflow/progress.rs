use super::super::context::{
    canvas_video_workflow_context, is_terminal_generated_video_status, normalized_provider_job_id,
    video_provider_workflow_business_key, video_provider_workflow_fingerprint,
};
use crate::AppState;
use a4n_core::models::ai::generated_video;
use a4n_orchestrator::workflow::{
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING,
    WorkflowProgressEventInput,
};
use serde_json::json;

pub(in crate::actions::video_generation::job_lifecycle) async fn record_video_provider_progress(
    state: &AppState,
    video: &generated_video::Model,
    current: u32,
    total: u32,
    message: &str,
) -> Result<i64, String> {
    if is_terminal_generated_video_status(&video.status) {
        return Err("视频已进入终态，不能继续记录 provider 进度".to_string());
    }
    let Some(job_id) = normalized_provider_job_id(video.provider_job_id.as_deref()) else {
        return Err("视频缺少 provider_job_id，不能记录 provider 进度".to_string());
    };
    let run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: video_provider_workflow_business_key(video.id),
            workflow_kind: "video_provider_job".to_string(),
            project_id: Some(video.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("generated_video".to_string()),
            scope_id: Some(video.id),
            chapter_id: video.chapter_id,
            mode: video.workflow_mode.clone(),
            input_fingerprint_json: Some(video_provider_workflow_fingerprint(video, job_id)),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("provider_poll".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    state
        .orchestrator
        .record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id: run.id,
            task_id: None,
            current,
            total,
            message,
            meta: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    record_canvas_video_provider_progress(state, video, current, total, message).await?;
    Ok(run.id)
}

async fn record_canvas_video_provider_progress(
    state: &AppState,
    video: &generated_video::Model,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), String> {
    let Some(canvas) = canvas_video_workflow_context(video)? else {
        return Ok(());
    };
    let step_key = format!("canvas_node:{}", canvas.node_id);
    state
        .orchestrator
        .mark_workflow_current_step(canvas.workflow_run_id, &step_key)
        .await
        .map_err(|err| err.to_string())?;
    state
        .orchestrator
        .upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: canvas.workflow_run_id,
            step_key,
            step_order: 0,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "generated_video_id": video.id,
                "provider_job_id": video.provider_job_id,
                "graph_id": canvas.graph_id,
                "node_id": canvas.node_id,
            })),
            checkpoint_json: None,
            artifacts_json: Some(json!({
                "data_type": "video",
                "generated_video_id": video.id,
                "status": video.status,
                "provider_job_id": video.provider_job_id,
            })),
            generation_run_ids_json: None,
            error_message: None,
            completed: false,
        })
        .await
        .map_err(|err| err.to_string())?;
    state
        .orchestrator
        .record_workflow_progress_event(WorkflowProgressEventInput {
            workflow_run_id: canvas.workflow_run_id,
            task_id: None,
            current,
            total,
            message,
            meta: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
