use super::super::contracts::CanvasRenderedPrompt;
use super::super::rules::canvas_config_string;
use super::context::collect_canvas_text_source_context;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) async fn build_canvas_prompt_artifact_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasRenderedPrompt, String> {
    let sources = collect_canvas_text_source_context(
        state,
        workflow_run_id,
        node,
        document,
        "RenderTextNode",
    )
    .await?;
    let template = canvas_config_string(&node.config_json, &["template", "template_text"])
        .ok_or_else(|| "RenderTextNode 缺少模板".to_string())?;
    let prompt_text = template
        .replace("{{context}}", &sources.text)
        .replace("{{input}}", &sources.text)
        .replace("{{content}}", &sources.text);
    if prompt_text.trim().is_empty() {
        return Err("RenderTextNode 缺少可渲染的文本内容".to_string());
    }
    let prompt_kind = canvas_config_string(&node.config_json, &["output_kind", "prompt_kind"])
        .unwrap_or_else(|| "canvas_prompt".to_string());
    Ok(CanvasRenderedPrompt {
        prompt_kind,
        prompt_text,
        canvas_text_asset_id: sources.canvas_text_asset_id,
        source_node_ids: sources.source_node_ids,
    })
}
