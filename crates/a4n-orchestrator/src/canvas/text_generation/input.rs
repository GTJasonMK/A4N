use a4n_core::models::{canvas::canvas_node, generation::workflow_checkpoint};
use a4n_db::canvas::CanvasGraphRecordDocument;

use crate::canvas::{
    canvas_config_f32, canvas_config_i64, canvas_config_string,
    source_text::collect_canvas_text_source_context,
};

use super::contracts::{CanvasLlmTextInput, CanvasRenderedPrompt};

pub(super) fn build_canvas_llm_text_input(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
) -> Result<CanvasLlmTextInput, String> {
    let sources = collect_canvas_text_source_context(node, document, checkpoints, "LlmTextNode")?;
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

    Ok(CanvasLlmTextInput {
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

pub(super) fn build_canvas_rendered_prompt(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
) -> Result<CanvasRenderedPrompt, String> {
    let sources =
        collect_canvas_text_source_context(node, document, checkpoints, "RenderTextNode")?;
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
