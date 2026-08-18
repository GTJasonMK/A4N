use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_llm::provider::is_truncated_finish_reason;
use serde_json::Value;

use crate::{OrchestratorContext, structured_llm::parse_strict_json_response};
use a4n_prompt::rendering::PromptRenderer;

use super::super::contracts::{
    MANGA_PIPELINE_MAX_TOKENS_PROFILE, MangaLlmCallInput, MangaPromptGenerationTrace,
};
use super::checkpoint::load_manga_restored_llm_checkpoint;
use super::request::prepare_manga_llm_call;

pub(in crate::manga) struct MangaRenderTraceInput<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub prompt_name: &'a str,
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub call_key: String,
    pub call_order: i32,
    pub restore_from_checkpoint: bool,
    pub variables: &'a HashMap<String, Value>,
    pub user_content: String,
    pub temperature: f32,
}

impl OrchestratorContext {
    pub(in crate::manga) async fn render_and_chat_trace(
        &self,
        input: MangaRenderTraceInput<'_>,
    ) -> Result<MangaPromptGenerationTrace, AppError> {
        let MangaRenderTraceInput {
            prompt_renderer,
            prompt_name,
            project_id,
            workflow_run_id,
            call_key,
            call_order,
            restore_from_checkpoint,
            variables,
            user_content,
            temperature,
        } = input;
        let prepared = prepare_manga_llm_call(MangaLlmCallInput {
            prompt_renderer,
            prompt_name,
            project_id,
            workflow_run_id: Some(workflow_run_id),
            call_key,
            call_order,
            variables,
            user_content,
            temperature,
        })
        .await?;
        let (response_text, usage, finish_reason, restored_parsed_output) =
            if restore_from_checkpoint
                && let Some(restored) = load_manga_restored_llm_checkpoint(
                    self,
                    workflow_run_id,
                    &prepared.call_key,
                    &prepared.input_fingerprint_json,
                )
                .await?
            {
                (
                    restored.response_text,
                    restored.usage,
                    restored.finish_reason,
                    Some(restored.parsed_output),
                )
            } else {
                let response = self.stream_complete_llm(prepared.request).await?;
                if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                    return Err(AppError::InvalidArgument(format!(
                        "{prompt_name} 输出被 max_tokens 截断，未形成完整 JSON。当前使用 token 档位 {MANGA_PIPELINE_MAX_TOKENS_PROFILE}，请减少本章输入规模或增大该档位后重试。"
                    )));
                }
                (
                    response.message.content,
                    response.usage,
                    response.finish_reason,
                    None,
                )
            };
        Ok(MangaPromptGenerationTrace {
            prompt_name: prompt_name.to_string(),
            request_json: prepared.request_json,
            messages_json: prepared.messages_json,
            response_text,
            workflow_run_id,
            call_key: prepared.call_key,
            call_order: prepared.call_order,
            input_fingerprint_json: prepared.input_fingerprint_json,
            usage,
            finish_reason,
            restored_parsed_output,
        })
    }
}

pub(in crate::manga) fn parse_and_validate_manga_llm_output<F, T>(
    trace: &MangaPromptGenerationTrace,
    validate: F,
) -> Result<T, AppError>
where
    F: FnOnce(&Value) -> Result<T, AppError>,
{
    let parsed = parse_strict_json_response::<Value>(&trace.response_text, &trace.prompt_name)?;
    validate(&parsed)
}
