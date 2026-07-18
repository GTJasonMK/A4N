use super::super::super::execution::{finish_canvas_run_failed, record_canvas_node_checkpoint};
use super::super::super::inputs::{build_canvas_trim_input_for_run, first_canvas_text_asset_id};
use super::super::super::node_lifecycle::{
    CanvasRunHandle, canvas_workflow_run_is_cancelled, finish_canvas_node_cancelled,
    finish_canvas_node_success, run_canvas_assembly_task, start_canvas_node_run,
};
use super::super::super::output_nodes::sync_completed_canvas_video_asset;
use super::super::super::persistence::{
    FreeformCanvasSpecInput, mark_canvas_generation_spec_status,
    upsert_freeform_canvas_generation_spec,
};
use super::super::super::{CanvasGraphDocument, CanvasRunSummary};
use crate::AppState;
use crate::actions::video_records::trim_video;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::workflow::{
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use serde_json::json;

pub(in crate::actions::canvas_workflow) async fn run_canvas_trim_video_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let (video_id, start_secs, end_secs, source_node_ids) =
        build_canvas_trim_input_for_run(Some(state), upstream_run_id, node, document).await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "video_id": video_id,
        "start_secs": start_secs,
        "end_secs": end_secs,
        "source_node_ids": source_node_ids});
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_video",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "video_id": video_id,
                "start_secs": start_secs,
                "end_secs": end_secs,
                "source_node_ids": source_node_ids}),
            params_json: json!({
                "workflow": "trim_video",
                "node_config": node.config_json}),
        },
    )
    .await?;
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    match run_canvas_assembly_task(state, project_id, run.workflow_run_id, || async {
        trim_video(state, user_id, video_id, start_secs, end_secs).await
    })
    .await
    {
        Ok(video) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let media_asset_id = sync_completed_canvas_video_asset(
                state,
                project_id,
                video.id,
                generation_spec.as_ref().map(|spec| spec.id),
                "canvas_trim",
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
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                    "media_asset_id": media_asset_id})),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, mode).await
        }
        Err(error) => {
            if canvas_workflow_run_is_cancelled(state, run.workflow_run_id).await? {
                return finish_canvas_node_cancelled(
                    state,
                    run,
                    project_id,
                    graph_id,
                    mode,
                    step_key,
                    input_json,
                    generation_spec.as_ref().map(|spec| spec.id),
                )
                .await;
            }
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            }
            finish_canvas_run_failed(
                state,
                run.workflow_run_id,
                project_id,
                graph_id,
                mode,
                step_key,
                input_json,
                error,
            )
            .await
        }
    }
}
