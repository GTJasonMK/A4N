use super::super::checkpoint::{SaveCanvasLlmTextCheckpointInput, save_canvas_llm_text_checkpoint};
use super::contracts::{CanvasLlmCallResult, ValidatedCanvasLlmText};
use crate::AppState;
use a4n_llm::provider::is_truncated_finish_reason;

pub(super) async fn validate_canvas_llm_text_call(
    state: &AppState,
    workflow_run_id: i64,
    call: CanvasLlmCallResult,
) -> Result<ValidatedCanvasLlmText, String> {
    let CanvasLlmCallResult {
        call_key,
        response_content,
        usage,
        finish_reason,
        restored_parsed_text,
        checkpoint_write,
    } = call;
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(
            "LlmTextNode 输出达到当前 max_tokens 上限，文本可能被截断；请调大输出档位或减少上游文本后重试"
                .to_string(),
        );
    }
    let transformed_text = response_content.trim().to_string();
    if let Some(restored_text) = restored_parsed_text.as_ref()
        && restored_text != &transformed_text
    {
        return Err(format!(
            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
        ));
    }
    if transformed_text.is_empty() {
        return Err("LlmTextNode 返回空文本".to_string());
    }
    if let Some(checkpoint_write) = checkpoint_write {
        save_canvas_llm_text_checkpoint(
            state,
            SaveCanvasLlmTextCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: checkpoint_write.call_order,
                input_fingerprint_json: checkpoint_write.input_fingerprint_json,
                response_content: &response_content,
                transformed_text: &transformed_text,
                usage: usage.as_ref(),
                finish_reason: finish_reason.as_deref(),
            },
        )
        .await
        .map_err(|err| format!("LlmTextNode 保存调用级 checkpoint 失败: {err}"))?;
    }
    Ok(ValidatedCanvasLlmText {
        transformed_text,
        usage,
        finish_reason,
    })
}
