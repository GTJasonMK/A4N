use serde_json::{Value, json};

use a4n_llm::provider::{UsageInfo, is_truncated_finish_reason};

use crate::AppState;
use crate::actions::workflow_checkpoint::{
    CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json,
};

pub(super) struct CanvasLlmTextCheckpoint {
    pub(super) response_content: String,
    pub(super) parsed_text: String,
    pub(super) usage: Option<UsageInfo>,
    pub(super) finish_reason: Option<String>,
}

pub(super) struct SaveCanvasLlmTextCheckpointInput<'a> {
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) call_order: i32,
    pub(super) input_fingerprint_json: Value,
    pub(super) response_content: &'a str,
    pub(super) transformed_text: &'a str,
    pub(super) usage: Option<&'a UsageInfo>,
    pub(super) finish_reason: Option<&'a str>,
}

pub(super) async fn load_canvas_llm_text_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<CanvasLlmTextCheckpoint>, String> {
    let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
        .await
        .map_err(|err| err.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json =
        required_llm_call_checkpoint_json(checkpoint, call_key).map_err(|err| err.to_string())?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    let parsed_text = parsed_output
        .get("text")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.parsed_output.text 必须是字符串"))?;
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|err| format!("{call_key}.usage 反序列化失败: {err}"))?;
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| format!("{call_key}.finish_reason 必须是字符串或 null"))?
                .to_string(),
        ),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        ));
    }
    Ok(Some(CanvasLlmTextCheckpoint {
        response_content,
        parsed_text,
        usage,
        finish_reason,
    }))
}

pub(super) async fn save_canvas_llm_text_checkpoint(
    state: &AppState,
    input: SaveCanvasLlmTextCheckpointInput<'_>,
) -> Result<(), String> {
    let usage_json = input
        .usage
        .map(serde_json::to_value)
        .transpose()
        .map_err(|err| format!("LlmTextNode 用量序列化失败: {err}"))?;
    state
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id: input.workflow_run_id,
            call_key: input.call_key,
            call_order: input.call_order,
            input_fingerprint_json: input.input_fingerprint_json,
            checkpoint_json: json!({
                "output": {
                    "text": input.transformed_text},
                "parsed_output": {
                    "text": input.transformed_text},
                "response_content": input.response_content,
                "usage": usage_json,
                "finish_reason": input.finish_reason}),
            artifacts_json: None,
        })
        .await
        .map_err(|err| err.to_string())
}
