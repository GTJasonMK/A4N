use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_orchestrator::portrait::normalize_portrait_prompt_response;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::super::super::workflow::{
    portrait_checkpoint_string, portrait_prompt_checkpoint_fingerprint,
};
use super::super::super::{AppState, PortraitProgress};
use super::super::{check_cancelled, report_progress};
use crate::actions::workflow_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    required_llm_call_checkpoint_json,
};

pub(super) struct InitialPortraitPrompt {
    pub(super) request: ChatRequest,
    pub(super) image_prompt: String,
    pub(super) raw_response_json: Value,
}

pub(super) async fn resolve_initial_portrait_prompt(
    state: &AppState,
    workflow_run_id: i64,
    workflow_fingerprint: &Value,
    mut request: ChatRequest,
    progress: &mut Option<PortraitProgress<'_>>,
    cancel_token: &CancellationToken,
) -> Result<InitialPortraitPrompt, String> {
    report_progress(progress, 1, 6, "正在渲染人物三视图拆解提示词模板")?;
    check_cancelled(cancel_token)?;
    report_progress(progress, 2, 6, "正在调用模型生成图片提示词")?;
    let call_key = "portrait_generation.prompt".to_string();
    let prompt_fingerprint =
        portrait_prompt_checkpoint_fingerprint(workflow_fingerprint, &request)?;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&prompt_fingerprint)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|err| err.to_string())?;
    let (image_prompt, raw_response_json) = if let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &prompt_fingerprint)
        .await
        .map_err(|err| err.to_string())?
    {
        report_progress(progress, 2, 6, "图片提示词已从断点恢复")?;
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
            .map_err(|err| err.to_string())?;
        let raw_prompt_response =
            portrait_checkpoint_string(&checkpoint_json, &call_key, "response_content")?;
        let image_prompt =
            portrait_checkpoint_string(&checkpoint_json, &call_key, "parsed_output")?;
        let reparsed = normalize_portrait_prompt_response(&raw_prompt_response)
            .map_err(|err| err.to_string())?;
        if reparsed != image_prompt {
            return Err(format!(
                "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
            ));
        }
        let usage = checkpoint_json
            .get("usage")
            .cloned()
            .ok_or_else(|| format!("{call_key}.usage 缺失"))?;
        let finish_reason = checkpoint_json
            .get("finish_reason")
            .cloned()
            .ok_or_else(|| format!("{call_key}.finish_reason 缺失"))?;
        let raw_response_json = json!({
            "text": raw_prompt_response,
            "usage": usage,
            "finish_reason": finish_reason});
        (image_prompt, raw_response_json)
    } else {
        let llm_response = state
            .llm
            .chat(request.clone())
            .await
            .map_err(|e| e.to_string())?;
        if is_truncated_finish_reason(llm_response.finish_reason.as_deref()) {
            return Err("人物三视图提示词输出达到当前 max_tokens 上限，提示词可能被截断；请在设置页调大“媒体提示词”输出档位，或减少角色上下文后重试".to_string());
        }
        let raw_prompt_response = llm_response.message.content.clone();
        let image_prompt =
            normalize_portrait_prompt_response(&raw_prompt_response).map_err(|e| e.to_string())?;
        let finish_reason = llm_response.finish_reason.clone();
        let response_usage_json = llm_response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|err| format!("人物三视图提示词 LLM 用量序列化失败: {err}"))?;
        let raw_response_json = json!({
            "text": raw_prompt_response.clone(),
            "usage": response_usage_json.clone(),
            "finish_reason": finish_reason});
        state
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: 1,
                input_fingerprint_json: prompt_fingerprint,
                checkpoint_json: json!({
                    "output": image_prompt.clone(),
                    "parsed_output": image_prompt.clone(),
                    "response_content": raw_prompt_response,
                    "usage": response_usage_json,
                    "finish_reason": finish_reason}),
                artifacts_json: None,
            })
            .await
            .map_err(|err| err.to_string())?;
        (image_prompt, raw_response_json)
    };
    Ok(InitialPortraitPrompt {
        request,
        image_prompt,
        raw_response_json,
    })
}
