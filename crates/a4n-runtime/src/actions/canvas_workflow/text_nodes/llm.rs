mod call;
mod completion;
mod contracts;
mod failure;
mod validation;

use call::execute_canvas_llm_text_call;
use completion::complete_canvas_llm_text_node;
use contracts::CanvasLlmNodeLifecycle;
use failure::fail_canvas_llm_text_node;
use validation::validate_canvas_llm_text_call;

use serde_json::json;

use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_RUNNING;

use crate::AppState;

use super::super::execution::finish_canvas_run_failed;
use super::super::inputs::build_canvas_llm_transform_input_for_run;
use super::super::node_lifecycle::{CanvasRunHandle, start_canvas_node_run};
use super::super::persistence::{FreeformCanvasSpecInput, upsert_freeform_canvas_generation_spec};
use super::super::{CanvasGraphDocument, CanvasRunSummary};

pub(in crate::actions::canvas_workflow) async fn run_canvas_llm_text_node(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let llm_input =
        build_canvas_llm_transform_input_for_run(Some(state), upstream_run_id, node, document)
            .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
    "project_id": project_id,
    "graph_id": graph_id,
    "node_id": node.id,
    "output_kind": llm_input.output_kind,
    "config_id": llm_input.config_id,
    "temperature": llm_input.temperature,
    "max_tokens": llm_input.max_tokens,
    "canvas_text_asset_id": llm_input.canvas_text_asset_id,
    "source_node_ids": llm_input.source_node_ids,
    "messages": [
        { "role": "system", "content": llm_input.system_prompt },
        { "role": "user", "content": llm_input.user_prompt }
    ]});
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
            canvas_text_asset_id: llm_input.canvas_text_asset_id,
            target_kind: "canvas_prompt",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "system_prompt": llm_input.system_prompt,
                "user_prompt": llm_input.user_prompt,
                "source_node_ids": llm_input.source_node_ids}),
            params_json: json!({
                "workflow": "llm_transform",
                "output_kind": llm_input.output_kind,
                "config_id": llm_input.config_id,
                "node_config": node.config_json}),
        },
    )
    .await?;
    let Some(spec) = spec else {
        return finish_canvas_run_failed(
            state,
            run.workflow_run_id,
            project_id,
            graph_id,
            mode,
            step_key,
            input_json,
            "LlmTextNode 当前只支持 freeform 视频项目".to_string(),
        )
        .await;
    };
    let lifecycle = CanvasLlmNodeLifecycle {
        run,
        project_id,
        graph_id,
        mode: mode.to_string(),
        step_key,
        input_json,
        generation_spec_id: spec.id,
    };
    let call = match execute_canvas_llm_text_call(
        state,
        project_id,
        graph_id,
        node,
        &llm_input,
        lifecycle.run.workflow_run_id,
        &lifecycle.step_key,
        mode,
    )
    .await
    {
        Ok(call) => call,
        Err(error) => return fail_canvas_llm_text_node(state, lifecycle, error).await,
    };
    let validated =
        match validate_canvas_llm_text_call(state, lifecycle.run.workflow_run_id, call).await {
            Ok(validated) => validated,
            Err(error) => return fail_canvas_llm_text_node(state, lifecycle, error).await,
        };
    complete_canvas_llm_text_node(state, lifecycle, &llm_input.output_kind, validated).await
}
