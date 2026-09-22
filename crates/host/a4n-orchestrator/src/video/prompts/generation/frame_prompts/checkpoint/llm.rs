use a4n_db::workflow::{
    self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput,
    UpsertWorkflowCheckpointInput,
};
use a4n_domain::workflow::WORKFLOW_STATUS_FAILED;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::workflow::workflow_checkpoint_contract_app_error;
use a4n_llm_trace::context_aware_chat_request_trace_json;

use super::super::{OrchestratorContext, RenderedVideoPromptTrace, VideoPromptLlmCallTraceContext};

pub(in crate::video::prompts::generation::frame_prompts) struct CompletedVideoLlmCheckpointInput<'a>
{
    pub(in crate::video::prompts::generation::frame_prompts) service: &'a OrchestratorContext,
    pub(in crate::video::prompts::generation::frame_prompts) workflow_run_id: i64,
    pub(in crate::video::prompts::generation::frame_prompts) step_key: String,
    pub(in crate::video::prompts::generation::frame_prompts) step_order: i32,
    pub(in crate::video::prompts::generation::frame_prompts) input_fingerprint_json: Value,
    pub(in crate::video::prompts::generation::frame_prompts) trace: &'a RenderedVideoPromptTrace,
    pub(in crate::video::prompts::generation::frame_prompts) parsed_output: Value,
    pub(in crate::video::prompts::generation::frame_prompts) artifacts_json: Option<Value>,
}

pub(in crate::video::prompts::generation::frame_prompts) async fn save_completed_video_llm_checkpoint(
    input: CompletedVideoLlmCheckpointInput<'_>,
) -> Result<(), AppError> {
    let CompletedVideoLlmCheckpointInput {
        service,
        workflow_run_id,
        step_key,
        step_order,
        input_fingerprint_json,
        trace,
        parsed_output,
        artifacts_json,
    } = input;
    let usage = trace.usage.as_ref().map(serde_json::to_value).transpose()?;
    service
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: step_key,
            call_order: step_order,
            input_fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output,
                response_content: trace.response_text.clone(),
                usage,
                finish_reason: trace.finish_reason.clone(),
                artifacts_json,
            },
        })
        .await
}

pub(in crate::video::prompts::generation::frame_prompts) struct FailedVideoLlmCheckpointInput<'a> {
    pub(in crate::video::prompts::generation::frame_prompts) workflow_run_id: i64,
    pub(in crate::video::prompts::generation::frame_prompts) call_key: String,
    pub(in crate::video::prompts::generation::frame_prompts) call_order: i32,
    pub(in crate::video::prompts::generation::frame_prompts) input_fingerprint_json: Value,
    pub(in crate::video::prompts::generation::frame_prompts) prompt_name: &'a str,
    pub(in crate::video::prompts::generation::frame_prompts) request: &'a ChatRequest,
    pub(in crate::video::prompts::generation::frame_prompts) response_content: Option<&'a str>,
    pub(in crate::video::prompts::generation::frame_prompts) error_message: String,
    pub(in crate::video::prompts::generation::frame_prompts) source: &'a str,
}

pub(in crate::video::prompts::generation::frame_prompts) async fn save_failed_video_llm_checkpoint(
    service: &OrchestratorContext,
    input: FailedVideoLlmCheckpointInput<'_>,
) -> Result<String, AppError> {
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&input.input_fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    let request_json = context_aware_chat_request_trace_json(input.request)?;
    service
        .upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: input.workflow_run_id,
            step_key: input.call_key.clone(),
            step_order: input.call_order,
            status: WORKFLOW_STATUS_FAILED.to_string(),
            input_fingerprint_sha256: Some(input_fingerprint_sha256.clone()),
            input_fingerprint_json: Some(input.input_fingerprint_json),
            checkpoint_json: Some(json!({
                "diagnostic_schema_version": 1,
                "diagnostic_kind": "video_llm_response_validation_failed",
                "source": input.source,
                "prompt_name": input.prompt_name,
                "call_key": input.call_key.clone(),
                "request": request_json,
                "response_content": input.response_content,
                "error_message": input.error_message.clone(),
            })),
            artifacts_json: Some(json!({
                "request_saved": true,
                "response_saved": input.response_content.is_some(),
            })),
            generation_run_ids_json: None,
            error_message: Some(input.error_message.clone()),
            completed: false,
        })
        .await?;
    Ok(format!(
        "已保存 LLM 请求/响应诊断：workflow_run_id={}, step_key={}, input_fingerprint_sha256={}",
        input.workflow_run_id, input.call_key, input_fingerprint_sha256
    ))
}

pub(in crate::video::prompts::generation::frame_prompts) fn video_llm_validation_error_with_diagnostic(
    err: AppError,
    diagnostic: Result<String, AppError>,
) -> AppError {
    let suffix = match diagnostic {
        Ok(message) => message,
        Err(save_err) => format!("LLM 请求/响应诊断保存失败：{save_err}"),
    };
    match err {
        AppError::InvalidArgument(message) => {
            AppError::InvalidArgument(format!("{message}；{suffix}"))
        }
        AppError::InvalidState { current, operation } => AppError::InvalidState {
            current,
            operation: format!("{operation}；{suffix}"),
        },
        AppError::Timeout(message) => AppError::Timeout(format!("{message}；{suffix}")),
        AppError::LlmError(message) => AppError::LlmError(format!("{message}；{suffix}")),
        other => AppError::InvalidArgument(format!("{other}；{suffix}")),
    }
}

pub(in crate::video::prompts::generation::frame_prompts) fn video_llm_call_trace_context(
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<VideoPromptLlmCallTraceContext, AppError> {
    Ok(VideoPromptLlmCallTraceContext {
        workflow_run_id,
        call_key: call_key.to_string(),
        input_fingerprint_sha256: workflow::llm_call_checkpoint_fingerprint_sha256(
            input_fingerprint_json,
        )
        .map_err(workflow_checkpoint_contract_app_error)?,
    })
}

pub(in crate::video::prompts::generation::frame_prompts) fn checkpoint_field<
    T: DeserializeOwned,
>(
    checkpoint: &Value,
    key: &str,
    context: &str,
) -> Result<T, AppError> {
    let value = checkpoint
        .get(key)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_field".to_string(),
            operation: format!("{context} 缺少字段 {key}"),
        })?;
    serde_json::from_value(value)
        .map_err(|err| AppError::InvalidArgument(format!("{context}.{key} 解析失败: {err}")))
}
