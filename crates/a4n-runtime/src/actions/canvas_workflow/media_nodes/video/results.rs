use super::super::super::CanvasRunSummary;
use super::super::super::execution::record_canvas_node_checkpoint;
use super::super::super::node_lifecycle::{
    canvas_workflow_run_is_cancelled, finish_canvas_node_cancelled, finish_canvas_node_success,
};
use super::super::super::output_nodes::sync_completed_canvas_video_asset;
use super::super::super::persistence::mark_canvas_generation_spec_status;
use super::context::CanvasVideoNodeContext;
use crate::AppState;
use crate::actions::video_generation::GeneratedVideoSummary;
use a4n_orchestrator::workflow::{
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use serde_json::json;

pub(super) async fn finish_canvas_video_node(
    state: &AppState,
    context: CanvasVideoNodeContext,
    result: Result<GeneratedVideoSummary, String>,
) -> Result<CanvasRunSummary, String> {
    let CanvasVideoNodeContext {
        project_id,
        graph_id,
        mode,
        step_key,
        input_json,
        run,
        generation_spec_id,
    } = context;

    match result {
        Ok(video) if video.status == WORKFLOW_STATUS_FAILED => {
            if let Some(spec_id) = generation_spec_id {
                mark_canvas_generation_spec_status(state, spec_id, WORKFLOW_STATUS_FAILED).await?;
            }
            let error = video
                .error_message
                .clone()
                .unwrap_or_else(|| "画布视频生成失败".to_string());
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_FAILED,
                input_json,
                Some(json!({
                    "generated_video_id": video.id,
                    "status": video.status,
                    "generation_spec_id": generation_spec_id})),
                Some(error.clone()),
                true,
            )
            .await?;
            let failed = state
                .orchestrator
                .mark_workflow_status(
                    run.workflow_run_id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(error.as_str()),
                )
                .await
                .map_err(|err| err.to_string())?;
            Ok(CanvasRunSummary {
                workflow_run_id: failed.id,
                project_id,
                graph_id,
                mode,
                status: failed.status,
                error_message: Some(error),
            })
        }
        Ok(video) if video.status == WORKFLOW_STATUS_RUNNING => {
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_RUNNING,
                input_json,
                Some(json!({
                    "generated_video_id": video.id,
                    "status": video.status,
                    "provider_job_id": video.provider_job_id,
                    "generation_spec_id": generation_spec_id})),
                None,
                false,
            )
            .await?;
            Ok(CanvasRunSummary {
                workflow_run_id: run.workflow_run_id,
                project_id,
                graph_id,
                mode,
                status: WORKFLOW_STATUS_RUNNING.to_string(),
                error_message: None,
            })
        }
        Ok(video) => {
            if let Some(spec_id) = generation_spec_id {
                mark_canvas_generation_spec_status(state, spec_id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let media_asset_id = sync_completed_canvas_video_asset(
                state,
                project_id,
                video.id,
                generation_spec_id,
                "generated_canvas_video",
            )
            .await?;
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_COMPLETED,
                input_json,
                Some(json!({
                    "data_type": "video",
                    "generated_video_id": video.id,
                    "status": video.status,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "generation_spec_id": generation_spec_id,
                    "media_asset_id": media_asset_id})),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, &mode).await
        }
        Err(error) => {
            if canvas_workflow_run_is_cancelled(state, run.workflow_run_id).await? {
                return finish_canvas_node_cancelled(
                    state,
                    run,
                    project_id,
                    graph_id,
                    &mode,
                    step_key,
                    input_json,
                    generation_spec_id,
                )
                .await;
            }
            if let Some(spec_id) = generation_spec_id {
                mark_canvas_generation_spec_status(state, spec_id, WORKFLOW_STATUS_FAILED).await?;
            }
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
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
                    run.workflow_run_id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(error.as_str()),
                )
                .await
                .map_err(|err| err.to_string())?;
            Ok(CanvasRunSummary {
                workflow_run_id: failed.id,
                project_id,
                graph_id,
                mode,
                status: failed.status,
                error_message: Some(error),
            })
        }
    }
}
