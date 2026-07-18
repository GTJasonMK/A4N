use super::super::{
    checkpoint::video_prompt_optimization_checkpoint_output,
    contracts::OptimizedVideoPromptFields,
    rules::{
        normalized_existing_optional_text, normalized_optimized_optional_text,
        optimized_motion_prompt, required_optimized_text, validate_optimized_prompt_fields,
    },
};
use crate::{
    AppState,
    actions::workflow_checkpoint::{
        CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json,
    },
};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_orchestrator::structured_llm::{
    StructuredRepairCheckpointContext, StructuredValueRepairInput,
};
use a4n_orchestrator::video::PromptOutputLanguage;
use serde_json::{Value, json};

pub(super) struct VideoPromptOptimizationCallResult {
    pub(super) response_text: String,
    pub(super) response_usage_json: Option<Value>,
    pub(super) response_finish_reason: Option<String>,
    pub(super) visual_prompt: String,
    pub(super) motion_prompt: Option<String>,
    pub(super) negative_prompt: Option<String>,
}

struct ValidatedVideoPromptOptimization {
    parsed: OptimizedVideoPromptFields,
    visual_prompt: String,
    motion_prompt: Option<String>,
    negative_prompt: Option<String>,
}

pub(super) async fn run_video_prompt_optimization_call(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
    prompt_output_language: PromptOutputLanguage,
    request: ChatRequest,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<VideoPromptOptimizationCallResult, String> {
    let (
        validated,
        response_text,
        checkpoint_response_text,
        response_usage_json,
        response_finish_reason,
    ) = if let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
        .await
        .map_err(|err| err.to_string())?
    {
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, call_key)
            .map_err(|err| err.to_string())?;
        let (optimized, response_usage_json, response_finish_reason) =
            video_prompt_optimization_checkpoint_output(&checkpoint_json, call_key)?;
        let response_text = serde_json::to_string(&optimized)
            .map_err(|err| format!("视频提示词优化断点结果序列化失败: {err}"))?;
        let validated =
            validate_video_prompt_optimization(prompt, optimized, prompt_output_language)?;
        (
            validated,
            response_text,
            None,
            response_usage_json,
            response_finish_reason,
        )
    } else {
        let repair_request = request.clone();
        let response = state
            .llm
            .chat(request)
            .await
            .map_err(|err| format!("优化视频提示词失败: {err}"))?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err("优化视频提示词输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“媒体提示词”输出档位，或缩短本次提示词后重试".to_string());
        }
        let checkpoint_response_text = response.message.content.clone();
        let response_usage_json = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|err| format!("LLM 用量序列化失败: {err}"))?;
        let response_finish_reason = response.finish_reason.clone();
        let parse_output = |value: &Value| {
            let optimized: OptimizedVideoPromptFields = serde_json::from_value(value.clone())
                .map_err(|err| {
                    a4n_core::error::AppError::InvalidArgument(format!(
                        "优化结果字段不符合要求: {err}"
                    ))
                })?;
            validate_video_prompt_optimization(prompt, optimized, prompt_output_language)
                .map_err(a4n_core::error::AppError::InvalidArgument)
        };
        let repaired = state
            .orchestrator
            .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
                request: repair_request,
                raw_response: &checkpoint_response_text,
                label: "视频提示词优化",
                schema: json!({
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["visual_prompt", "motion_prompt", "negative_prompt"],
                    "properties": {
                        "visual_prompt": {"type": "string"},
                        "motion_prompt": {"type": ["string", "null"]},
                        "negative_prompt": {"type": ["string", "null"]}
                    }
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
            serde_json::to_string(&repaired.final_value)
                .map_err(|err| format!("视频提示词优化结果序列化失败: {err}"))?,
            Some(checkpoint_response_text),
            response_usage_json,
            response_finish_reason,
        )
    };
    let ValidatedVideoPromptOptimization {
        parsed,
        visual_prompt,
        motion_prompt,
        negative_prompt,
    } = validated;
    if let Some(checkpoint_response_text) = checkpoint_response_text {
        state
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: call_key.to_string(),
                call_order: 1,
                input_fingerprint_json: input_fingerprint_json.clone(),
                checkpoint_json: json!({
                    "output": {
                        "visual_prompt": visual_prompt.clone(),
                        "motion_prompt": motion_prompt.clone(),
                        "negative_prompt": negative_prompt.clone()},
                    "parsed_output": {
                        "visual_prompt": parsed.visual_prompt,
                        "motion_prompt": parsed.motion_prompt,
                        "negative_prompt": parsed.negative_prompt},
                    "response_content": checkpoint_response_text,
                    "usage": response_usage_json.clone(),
                    "finish_reason": response_finish_reason.clone()}),
                artifacts_json: None,
            })
            .await
            .map_err(|err| err.to_string())?;
    }

    Ok(VideoPromptOptimizationCallResult {
        response_text,
        response_usage_json,
        response_finish_reason,
        visual_prompt,
        motion_prompt,
        negative_prompt,
    })
}

fn validate_video_prompt_optimization(
    prompt: &chapter_video_prompt::Model,
    parsed: OptimizedVideoPromptFields,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ValidatedVideoPromptOptimization, String> {
    let visual_prompt = required_optimized_text(&parsed.visual_prompt, "visual_prompt")?;
    let motion_prompt = optimized_motion_prompt(prompt, parsed.motion_prompt.clone())?;
    let negative_prompt = normalized_optimized_optional_text(parsed.negative_prompt.clone())
        .or_else(|| normalized_existing_optional_text(prompt.negative_prompt.as_deref()));
    validate_optimized_prompt_fields(
        prompt,
        &visual_prompt,
        motion_prompt.as_deref(),
        negative_prompt.as_deref(),
        prompt_output_language,
    )?;
    Ok(ValidatedVideoPromptOptimization {
        parsed,
        visual_prompt,
        motion_prompt,
        negative_prompt,
    })
}
