use a4n_core::error::AppError;
use a4n_db::workflow::{
    self, CompletedLlmCallCheckpointInput, UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_FAILED,
    upsert_workflow_checkpoint,
};
use a4n_llm::provider::{ChatRequest, UsageInfo};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};
use a4n_llm::trace::context_aware_chat_request_trace_json;

use super::super::{OrchestratorContext, RenderedVideoPromptTrace, VideoPromptLlmCallTraceContext};

pub(in crate::video::prompt_generation::frame_prompts) async fn load_completed_video_checkpoint(
    service: &OrchestratorContext,
    workflow_run_id: i64,
    step_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<Value>, AppError> {
    let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        service.db.connection(),
        workflow_run_id,
        step_key,
        input_fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?
    else {
        return Ok(None);
    };
    let checkpoint_json = workflow::required_llm_call_checkpoint_json(checkpoint, step_key)
        .map_err(workflow_checkpoint_contract_app_error)?;
    Ok(Some(checkpoint_json))
}

pub(in crate::video::prompt_generation::frame_prompts) struct CompletedVideoLlmCheckpointInput<'a> {
    pub(in crate::video::prompt_generation::frame_prompts) service: &'a OrchestratorContext,
    pub(in crate::video::prompt_generation::frame_prompts) workflow_run_id: i64,
    pub(in crate::video::prompt_generation::frame_prompts) step_key: String,
    pub(in crate::video::prompt_generation::frame_prompts) step_order: i32,
    pub(in crate::video::prompt_generation::frame_prompts) input_fingerprint_json: Value,
    pub(in crate::video::prompt_generation::frame_prompts) trace: &'a RenderedVideoPromptTrace,
    pub(in crate::video::prompt_generation::frame_prompts) parsed_output: Value,
    pub(in crate::video::prompt_generation::frame_prompts) extra_json: Value,
    pub(in crate::video::prompt_generation::frame_prompts) artifacts_json: Option<Value>,
}

pub(in crate::video::prompt_generation::frame_prompts) async fn save_completed_video_llm_checkpoint(
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
        extra_json,
        artifacts_json,
    } = input;
    let usage = trace.usage.as_ref().map(serde_json::to_value).transpose()?;
    let mut checkpoint_json = extra_json.as_object().cloned().ok_or_else(|| {
        AppError::InvalidArgument("视频 LLM checkpoint 扩展字段必须是 JSON 对象".to_string())
    })?;
    checkpoint_json.insert("output".to_string(), parsed_output.clone());
    checkpoint_json.insert("parsed_output".to_string(), parsed_output);
    checkpoint_json.insert(
        "response_content".to_string(),
        Value::String(trace.response_text.clone()),
    );
    checkpoint_json.insert("usage".to_string(), usage.unwrap_or(Value::Null));
    checkpoint_json.insert(
        "finish_reason".to_string(),
        trace
            .finish_reason
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .unwrap_or(Value::Null),
    );
    workflow::save_completed_llm_call_checkpoint(
        service.db.connection(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: step_key,
            call_order: step_order,
            input_fingerprint_json,
            checkpoint_json: Value::Object(checkpoint_json),
            artifacts_json,
        },
    )
    .await
    .map_err(workflow_write_app_error)
}

pub(in crate::video::prompt_generation::frame_prompts) struct FailedVideoLlmCheckpointInput<'a> {
    pub(in crate::video::prompt_generation::frame_prompts) workflow_run_id: i64,
    pub(in crate::video::prompt_generation::frame_prompts) call_key: String,
    pub(in crate::video::prompt_generation::frame_prompts) call_order: i32,
    pub(in crate::video::prompt_generation::frame_prompts) input_fingerprint_json: Value,
    pub(in crate::video::prompt_generation::frame_prompts) prompt_name: &'a str,
    pub(in crate::video::prompt_generation::frame_prompts) request: &'a ChatRequest,
    pub(in crate::video::prompt_generation::frame_prompts) response_content: Option<&'a str>,
    pub(in crate::video::prompt_generation::frame_prompts) error_message: String,
    pub(in crate::video::prompt_generation::frame_prompts) source: &'a str,
}

pub(in crate::video::prompt_generation::frame_prompts) async fn save_failed_video_llm_checkpoint(
    service: &OrchestratorContext,
    input: FailedVideoLlmCheckpointInput<'_>,
) -> Result<String, AppError> {
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&input.input_fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    let request_json = context_aware_chat_request_trace_json(input.request)?;
    upsert_workflow_checkpoint(
        service.db.connection(),
        UpsertWorkflowCheckpointInput {
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
        },
    )
    .await
    .map_err(workflow_write_app_error)?;
    Ok(format!(
        "已保存 LLM 请求/响应诊断：workflow_run_id={}, step_key={}, input_fingerprint_sha256={}",
        input.workflow_run_id, input.call_key, input_fingerprint_sha256
    ))
}

pub(in crate::video::prompt_generation::frame_prompts) fn video_llm_validation_error_with_diagnostic(
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

pub(in crate::video::prompt_generation::frame_prompts) fn video_llm_call_trace_context(
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

pub(in crate::video::prompt_generation::frame_prompts) fn checkpoint_response_content<'a>(
    checkpoint: &'a Value,
    context: &str,
) -> Result<&'a str, AppError> {
    checkpoint
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取{context}"),
        })
}

pub(in crate::video::prompt_generation::frame_prompts) fn ensure_checkpoint_parsed_output(
    checkpoint: &Value,
    parsed_output: &Value,
    context: &str,
) -> Result<(), AppError> {
    let checkpoint_output =
        checkpoint
            .get("parsed_output")
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_checkpoint_parsed_output".to_string(),
                operation: format!("读取{context}"),
            })?;
    if checkpoint_output != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验{context}"),
        });
    }
    Ok(())
}

pub(in crate::video::prompt_generation::frame_prompts) fn checkpoint_usage(
    checkpoint: &Value,
    context: &str,
) -> Result<Option<UsageInfo>, AppError> {
    checkpoint_field(checkpoint, "usage", context)
}

pub(in crate::video::prompt_generation::frame_prompts) fn checkpoint_finish_reason(
    checkpoint: &Value,
    context: &str,
) -> Result<Option<String>, AppError> {
    checkpoint_field(checkpoint, "finish_reason", context)
}

pub(in crate::video::prompt_generation::frame_prompts) fn checkpoint_field<T: DeserializeOwned>(
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
