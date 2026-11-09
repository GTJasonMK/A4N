use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::is_truncated_finish_reason;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    structured_llm::{ensure_checkpoint_parsed_output_matches, parse_strict_json_response},
    workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput},
};
use a4n_prompt::rendering::PromptRenderer;

use super::super::contracts::{MangaLlmCallInput, MangaPromptGenerationTrace};
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
    pub user_content: a4n_llm_context::LlmContextManifest,
    pub temperature: f32,
    pub cancel_token: Option<&'a CancellationToken>,
    pub stream_id: String,
    pub stream_reporter: Option<&'a (dyn Fn(&str, &str) + Send + Sync)>,
}

impl OrchestratorContext {
    pub(in crate::manga) async fn render_and_validate_manga_call<T, F>(
        &self,
        input: MangaRenderTraceInput<'_>,
        validate: F,
    ) -> Result<(MangaPromptGenerationTrace, T), AppError>
    where
        T: serde::Serialize,
        F: FnMut(&Value) -> Result<T, AppError>,
    {
        self.render_and_validate_manga_call_observed(input, validate, None)
            .await
    }

    pub(in crate::manga) async fn render_and_validate_manga_call_observed<T, F>(
        &self,
        input: MangaRenderTraceInput<'_>,
        mut validate: F,
        on_correction: Option<&(dyn Fn() + Send + Sync)>,
    ) -> Result<(MangaPromptGenerationTrace, T), AppError>
    where
        T: serde::Serialize,
        F: FnMut(&Value) -> Result<T, AppError>,
    {
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
            cancel_token,
            stream_id,
            stream_reporter,
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
                let scoped_reporter = |delta: &str| {
                    if let Some(reporter) = stream_reporter {
                        reporter(&stream_id, delta);
                    }
                };
                let stream_sink =
                    stream_reporter.map(|_| &scoped_reporter as &(dyn Fn(&str) + Send + Sync));
                let response = self
                    .stream_complete_llm_with_sink(
                        prepared.request.clone(),
                        stream_sink,
                        cancel_token,
                    )
                    .await?;
                if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                    return Err(AppError::InvalidArgument(format!(
                        "{prompt_name} 输出达到模型当前 max_tokens 上限，未形成完整 JSON；请减少本次输入规模或提高模型配置的最大输出后重试。"
                    )));
                }
                (
                    response.message.content,
                    response.usage,
                    response.finish_reason,
                    None,
                )
            };
        let mut trace = MangaPromptGenerationTrace {
            restored_from_checkpoint: restored_parsed_output.is_some(),
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
        };
        let mut parse = |raw: &str| {
            parse_strict_json_response::<Value>(raw, prompt_name).and_then(|value| {
                let output = validate(&value)?;
                let parsed_output = serde_json::to_value(&output)?;
                if let Some(saved) = restored_parsed_output.as_ref() {
                    ensure_checkpoint_parsed_output_matches(
                        saved,
                        &parsed_output,
                        &format!("漫画管线 {}", trace.call_key),
                    )?;
                }
                Ok((output, parsed_output))
            })
        };
        let parsed = if restored_parsed_output.is_some() {
            parse(&trace.response_text)
        } else {
            let initial = a4n_llm_provider_contract::provider::ChatResponse {
                message: a4n_llm_provider_contract::provider::ChatMessage::assistant(
                    &trace.response_text,
                ),
                usage: trace.usage.clone(),
                finish_reason: trace.finish_reason.clone(),
                reasoning_content: None,
            };
            let correction_stream_id = format!("{stream_id}.correction");
            let correction_sink = |delta: &str| {
                if let Some(reporter) = stream_reporter {
                    reporter(&correction_stream_id, delta);
                }
            };
            self.validate_llm_response_observed(
                &prepared.request,
                initial,
                prompt_name,
                cancel_token,
                parse,
                on_correction,
                stream_reporter.map(|_| &correction_sink as &(dyn Fn(&str) + Send + Sync)),
            )
            .await
            .map(|(response, parsed)| {
                trace.response_text = response.message.content;
                trace.usage = response.usage;
                trace.finish_reason = response.finish_reason;
                parsed
            })
        };
        let (output, parsed_output) = if restored_parsed_output.is_some() {
            self.validate_llm_call_output(
                trace.workflow_run_id,
                &trace.call_key,
                &trace.input_fingerprint_json,
                parsed,
            )
            .await?
        } else {
            parsed?
        };
        if restored_parsed_output.is_none() {
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: trace.call_key.clone(),
                call_order: trace.call_order,
                input_fingerprint_json: trace.input_fingerprint_json.clone(),
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output,
                    response_content: trace.response_text.clone(),
                    usage: trace.usage.as_ref().map(serde_json::to_value).transpose()?,
                    finish_reason: trace.finish_reason.clone(),
                    artifacts_json: None,
                },
            })
            .await?;
        }
        Ok((trace, output))
    }
}
