use super::contracts::{CanvasNodeRequestPreview, PreviewCanvasNodeRequestInput};
use super::fallback::blocked_canvas_node_request_preview;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;
use serde_json::json;

pub(super) async fn preview_canvas_llm_node(
    canvas: &a4n_orchestrator::OrchestratorContext,
    input: &PreviewCanvasNodeRequestInput,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasNodeRequestPreview, String> {
    let llm_input = match canvas
        .prepare_canvas_llm_text_input_validated(None, node, document)
        .await
    {
        Ok(llm_input) => llm_input,
        Err(reason) => {
            return Ok(blocked_canvas_node_request_preview(
                input,
                node,
                document,
                reason.to_string(),
            ));
        }
    };
    Ok(CanvasNodeRequestPreview {
        project_id: input.project_id,
        graph_id: input.graph_id,
        node_id: input.node_id,
        node_kind: node.node_kind.clone(),
        note: "已构造 LlmTextNode 的 LLM 请求预览；预览不会调用外部模型。".to_string(),
        request: json!({
            "config_id": llm_input.config_id,
            "output_kind": llm_input.output_kind,
            "temperature": llm_input.temperature,
            "max_tokens": llm_input.max_tokens,
            "messages": [
                { "role": "system", "content": llm_input.system_prompt },
                { "role": "user", "content": llm_input.user_prompt }
            ],
            "source_node_ids": llm_input.source_node_ids,
            "canvas_text_asset_id": llm_input.canvas_text_asset_id
        }),
    })
}
