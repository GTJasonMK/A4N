use a4n_core::models::ai::generated_video;
use a4n_orchestrator::workflow::{
    UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
};
use a4n_video::VideoGenerationResult;
use serde_json::json;

use super::super::context::{canvas_video_workflow_context, is_canvas_video_record};
use super::super::workflow::mark_video_provider_workflow;
use super::video::save_completed_video_result;
use crate::AppState;
use crate::actions::video_generation::media::prompt_id_from_video;
use crate::actions::video_generation::save_completed_canvas_video_result;

pub(in crate::actions::video_generation::job_lifecycle) async fn save_completed_provider_result_for_video(
    state: &AppState,
    video: &generated_video::Model,
    result: VideoGenerationResult,
) -> Result<generated_video::Model, String> {
    save_completed_provider_result_for_video_with_canvas_workflow_mode(state, video, result, true)
        .await
}

pub(in crate::actions::video_generation::job_lifecycle) async fn save_completed_provider_result_for_video_with_canvas_workflow_mode(
    state: &AppState,
    video: &generated_video::Model,
    result: VideoGenerationResult,
    finalize_canvas_workflow: bool,
) -> Result<generated_video::Model, String> {
    if is_canvas_video_record(video) {
        let updated = save_completed_canvas_video_result(state, video, result).await?;
        mark_video_provider_workflow(state, &updated, WORKFLOW_STATUS_COMPLETED, false, None)
            .await?;
        if finalize_canvas_workflow {
            mark_canvas_video_workflow_from_video(state, &updated, WORKFLOW_STATUS_COMPLETED, None)
                .await?;
        }
        Ok(updated)
    } else {
        save_completed_video_result(state, video, prompt_id_from_video(video), result).await
    }
}

pub(in crate::actions::video_generation::job_lifecycle) async fn mark_canvas_video_workflow_from_video(
    state: &AppState,
    video: &generated_video::Model,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    let Some(canvas) = canvas_video_workflow_context(video)? else {
        return Ok(());
    };
    let workflow_run_id = canvas.workflow_run_id;
    let graph_id = canvas.graph_id;
    let node_id = canvas.node_id;
    let step_key = format!("canvas_node:{node_id}");
    state
        .orchestrator
        .upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key,
            step_order: 0,
            status: status.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "generated_video_id": video.id,
                "provider_job_id": video.provider_job_id,
                "graph_id": graph_id,
                "node_id": node_id,
            })),
            checkpoint_json: None,
            artifacts_json: Some(json!({
                "generated_video_id": video.id,
                "status": video.status,
                "file_path": video.file_path,
                "thumbnail_path": video.thumbnail_path,
                "provider_job_id": video.provider_job_id,
            })),
            generation_run_ids_json: None,
            error_message: error_message.map(str::to_string),
            completed: matches!(status, WORKFLOW_STATUS_COMPLETED | WORKFLOW_STATUS_FAILED),
        })
        .await
        .map_err(crate::actions::app_error_message)?;
    state
        .orchestrator
        .mark_workflow_status(
            workflow_run_id,
            status,
            status == WORKFLOW_STATUS_FAILED,
            error_message,
        )
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
