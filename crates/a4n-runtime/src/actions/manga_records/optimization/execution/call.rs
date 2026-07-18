use crate::{
    AppState,
    actions::{
        manga_records::{
            access::required_trimmed,
            optimization::checkpoint::manga_prompt_optimization_checkpoint_output,
        },
        workflow_checkpoint::{CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json},
    },
};
use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_orchestrator::structured_llm::{
    StructuredRepairCheckpointContext, StructuredValueRepairInput,
};
use serde_json::{Value, json};

struct ValidatedMangaPromptOptimization {
    parsed_image_prompt: String,
    image_prompt: String,
}

pub(super) async fn run_manga_prompt_optimization_call(
    state: &AppState,
    request: ChatRequest,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<String, String> {
    let (validated, checkpoint_response_text, response_usage_json, response_finish_reason) =
        if let Some(checkpoint) = state
            .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
            .await
            .map_err(|err| err.to_string())?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, call_key)
                .map_err(|err| err.to_string())?;
            let (optimized, usage, finish_reason) =
                manga_prompt_optimization_checkpoint_output(&checkpoint_json, call_key)?;
            (
                validate_manga_prompt_optimization(optimized)?,
                None,
                usage,
                finish_reason,
            )
        } else {
            let repair_request = request.clone();
            let response = state
                .llm
                .chat(request)
                .await
                .map_err(|err| format!("优化漫画提示词失败: {err}"))?;
            if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                return Err("优化漫画提示词输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“媒体提示词”输出档位，或缩短本次提示词后重试".to_string());
            }
            let checkpoint_response_text = response.message.content.clone();
            let response_usage_json = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|err| format!("LLM 用量序列化失败: {err}"))?;
            let parse_output = |value: &Value| {
                let optimized: super::super::super::OptimizedMangaPromptFields =
                    serde_json::from_value(value.clone()).map_err(|err| {
                        a4n_core::error::AppError::InvalidArgument(format!(
                            "漫画提示词优化结果字段不符合要求: {err}"
                        ))
                    })?;
                validate_manga_prompt_optimization(optimized)
                    .map_err(a4n_core::error::AppError::InvalidArgument)
            };
            let repaired = state
                .orchestrator
                .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
                    request: repair_request,
                    raw_response: &checkpoint_response_text,
                    label: "漫画提示词优化",
                    schema: json!({
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["image_prompt"],
                        "properties": {"image_prompt": {"type": "string"}}
                    }),
                    checkpoint: Some(StructuredRepairCheckpointContext {
                        workflow_run_id,
                        base_call_key: call_key.to_string(),
                        call_order: 1,
                    }),
                    cancel_token: None,
                    progress: None,
                    parse_output: &parse_output,
                })
                .await
                .map_err(|err| err.to_string())?;
            (
                repaired.output,
                Some(checkpoint_response_text),
                response_usage_json,
                response.finish_reason,
            )
        };

    let ValidatedMangaPromptOptimization {
        parsed_image_prompt,
        image_prompt,
    } = validated;
    if let Some(checkpoint_response_text) = checkpoint_response_text {
        state
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: call_key.to_string(),
                call_order: 1,
                input_fingerprint_json: input_fingerprint_json.clone(),
                checkpoint_json: json!({
                    "output": {"image_prompt": image_prompt},
                    "parsed_output": {"image_prompt": parsed_image_prompt},
                    "response_content": checkpoint_response_text,
                    "usage": response_usage_json,
                    "finish_reason": response_finish_reason,
                }),
                artifacts_json: None,
            })
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(image_prompt)
}

fn validate_manga_prompt_optimization(
    optimized: super::super::super::OptimizedMangaPromptFields,
) -> Result<ValidatedMangaPromptOptimization, String> {
    let image_prompt =
        required_trimmed(&optimized.image_prompt, "优化结果 image_prompt")?.to_string();
    Ok(ValidatedMangaPromptOptimization {
        parsed_image_prompt: optimized.image_prompt,
        image_prompt,
    })
}
