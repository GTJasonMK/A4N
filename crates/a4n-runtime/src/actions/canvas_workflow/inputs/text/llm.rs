use super::super::contracts::CanvasLlmTransformInput;
use super::super::rules::{canvas_config_f32, canvas_config_i64, canvas_config_string};
use super::context::collect_canvas_text_source_context;
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) async fn build_canvas_llm_transform_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasLlmTransformInput, String> {
    let sources =
        collect_canvas_text_source_context(state, workflow_run_id, node, document, "LlmTextNode")
            .await?;
    let config_id = canvas_config_i64(&node.config_json, "llm_config_id");
    let instruction = canvas_config_string(&node.config_json, &["instruction", "prompt"])
        .unwrap_or_else(|| {
            "将输入内容改写为适合视频画布后续节点使用的结构化创作文本。".to_string()
        });
    if sources.text.trim().is_empty() {
        return Err("LlmTextNode 缺少输入文本".to_string());
    }
    let output_kind = canvas_config_string(&node.config_json, &["output_kind", "text_kind"])
        .unwrap_or_else(|| "shot_script".to_string());
    let system_prompt = format!(
        "你是 A4N 视频画布的文本转换节点。请根据用户指令转换输入文本，输出类型为 {output_kind}。只返回转换后的正文，不要解释过程。"
    );
    let user_prompt = format!("指令：{instruction}\n\n输入：\n{}", sources.text);
    let temperature = canvas_config_f32(&node.config_json, "temperature");
    let max_tokens = canvas_config_i64(&node.config_json, "max_tokens")
        .and_then(|value| usize::try_from(value).ok());

    Ok(CanvasLlmTransformInput {
        system_prompt,
        user_prompt,
        instruction,
        source_context: sources.text,
        output_kind,
        temperature,
        max_tokens,
        config_id,
        canvas_text_asset_id: sources.canvas_text_asset_id,
        source_node_ids: sources.source_node_ids,
    })
}
