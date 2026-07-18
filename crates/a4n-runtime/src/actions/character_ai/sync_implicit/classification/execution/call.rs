use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};
use serde_json::{Value, json};

use crate::AppState;
use crate::actions::character_ai::implicit_classification::parse_implicit_classification_output;
use crate::actions::character_ai::sync_implicit::checkpoint::{
    optional_checkpoint_string, required_checkpoint_string,
};
use crate::actions::workflow_checkpoint::{
    CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json,
};

pub(super) async fn run_implicit_classification_call(
    state: &AppState,
    request: ChatRequest,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
    behavior_index: usize,
    implicit_attributes: &Value,
) -> Result<Value, String> {
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, call_key)
        .await
        .map_err(|err| err.to_string())?;

    let (response_text, response_usage_json, response_finish_reason, restored_parsed_output) =
        if let Some(checkpoint) = state
            .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
            .await
            .map_err(|err| err.to_string())?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, call_key)
                .map_err(|err| err.to_string())?;
            let response_text =
                required_checkpoint_string(&checkpoint_json, call_key, "response_content")?;
            let finish_reason = optional_checkpoint_string(&checkpoint_json, "finish_reason")?;
            if is_truncated_finish_reason(finish_reason.as_deref()) {
                return Err("行为隐性属性分类 checkpoint 标记为截断响应，不能用于恢复".to_string());
            }
            let parsed_output = checkpoint_json
                .get("parsed_output")
                .cloned()
                .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
            let usage = checkpoint_json
                .get("usage")
                .filter(|value| !value.is_null())
                .cloned();
            (response_text, usage, finish_reason, Some(parsed_output))
        } else {
            let response = state
                .orchestrator
                .chat_llm(request)
                .await
                .map_err(|err| err.to_string())?;
            if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                return Err("行为隐性属性分类输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次分类上下文后重试".to_string());
            }
            let usage = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|err| format!("行为隐性属性分类 LLM 用量序列化失败: {err}"))?;
            (
                response.message.content,
                usage,
                response.finish_reason,
                None,
            )
        };

    let parsed = parse_implicit_classification_output(&response_text, implicit_attributes)
        .map_err(|err| format!("行为隐性属性分类输出不符合契约：{err}"))?;
    if let Some(parsed_output) = restored_parsed_output {
        if parsed_output != parsed {
            return Err(format!(
                "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
            ));
        }
    } else {
        let call_order = i32::try_from(behavior_index)
            .map_err(|_| format!("行为隐性属性分类索引超过 i32 范围：{behavior_index}"))?
            .checked_add(1000)
            .ok_or_else(|| {
                format!("行为隐性属性分类 call_order 超过 i32 范围：{behavior_index}")
            })?;
        state
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: call_key.to_string(),
                call_order,
                input_fingerprint_json: input_fingerprint_json.clone(),
                checkpoint_json: json!({
                    "output": parsed.clone(),
                    "parsed_output": parsed.clone(),
                    "response_content": response_text,
                    "usage": response_usage_json,
                    "finish_reason": response_finish_reason}),
                artifacts_json: None,
            })
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(parsed)
}
