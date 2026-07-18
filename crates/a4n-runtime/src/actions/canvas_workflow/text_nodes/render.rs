use serde_json::json;

use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_COMPLETED;

use crate::AppState;

use super::super::execution::record_canvas_node_checkpoint;
use super::super::inputs::build_canvas_prompt_artifact_input_for_run;
use super::super::node_lifecycle::{
    CanvasRunHandle, finish_canvas_node_success, start_canvas_node_run,
};
use super::super::persistence::{
    FreeformCanvasSpecInput, upsert_canvas_prompt_artifact, upsert_freeform_canvas_generation_spec,
};
use super::super::{CanvasGraphDocument, CanvasRunSummary};

pub(in crate::actions::canvas_workflow) async fn run_canvas_render_text_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let rendered =
        build_canvas_prompt_artifact_input_for_run(Some(state), upstream_run_id, node, document)
            .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "prompt_kind": rendered.prompt_kind,
        "prompt_text": rendered.prompt_text,
        "canvas_text_asset_id": rendered.canvas_text_asset_id,
        "source_node_ids": rendered.source_node_ids});
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

    let spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: rendered.canvas_text_asset_id,
            target_kind: "canvas_prompt",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_COMPLETED,
            source_material_json: json!({
                "prompt_text": rendered.prompt_text,
                "source_node_ids": rendered.source_node_ids}),
            params_json: json!({
                "workflow": "render_prompt",
                "prompt_kind": rendered.prompt_kind,
                "node_config": node.config_json}),
        },
    )
    .await?;
    let spec = spec.ok_or_else(|| "RenderTextNode 当前只支持 freeform 视频项目".to_string())?;
    let artifact =
        upsert_canvas_prompt_artifact(state, spec.id, &rendered.prompt_kind, &rendered.prompt_text)
            .await?;
    record_canvas_node_checkpoint(
        state,
        run.workflow_run_id,
        step_key,
        WORKFLOW_STATUS_COMPLETED,
        input_json,
        Some(json!({
            "data_type": "text",
            "text": artifact.prompt_text,
            "generation_spec_id": spec.id,
            "prompt_artifact_id": artifact.id,
            "prompt_kind": artifact.prompt_kind})),
        None,
        true,
    )
    .await?;
    finish_canvas_node_success(state, run, project_id, graph_id, mode).await
}
