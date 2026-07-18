use std::time::Instant;

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, ChatResponse};
use a4n_queue::priority::{TaskPriority, TaskType};

use super::super::budget::ensure_max_tokens_within_model_cap;
use super::super::config::{chat_embedded, chat_via_sidecar, require_sidecar_business_trace};
use super::super::contracts::DynamicLlmProvider;
use super::super::trace::{
    llm_error_kind, llm_trace_status_for_error, prepare_dynamic_llm_trace_context,
    record_dynamic_llm_trace,
};

impl DynamicLlmProvider {
    pub async fn chat_with_config_id(
        &self,
        config_id: Option<i64>,
        mut request: ChatRequest,
    ) -> Result<ChatResponse, AppError> {
        let profile = request.max_tokens_profile.clone();
        let temperature = request.temperature;
        let message_count = request.messages.len();
        let prompt_chars: usize = request.messages.iter().map(|msg| msg.content.len()).sum();
        let resolved_model = self.resolve_runtime_model_by_id(config_id).await?;
        let model_name = resolved_model.model_name().to_string();
        let model_max_output_tokens = resolved_model.capabilities.max_output_tokens;
        request.max_tokens = self.resolve_max_tokens(&request).await?;
        ensure_max_tokens_within_model_cap(
            request.max_tokens,
            profile.as_deref(),
            model_max_output_tokens,
            model_name.as_str(),
        )?;
        let planner = resolved_model.planner();
        let trace_context =
            prepare_dynamic_llm_trace_context(&self.orchestrator, &planner, &mut request).await?;
        let sidecar_call = match self.sidecar_client.clone() {
            Some(client) => Some((
                client,
                require_sidecar_business_trace(trace_context.as_ref(), "LLM sidecar chat 请求")?
                    .clone(),
            )),
            None => None,
        };
        tracing::debug!(
            target: "a4n::llm",
            model = %model_name,
            config_id = ?config_id,
            profile = ?profile,
            max_tokens = ?request.max_tokens,
            temperature = ?temperature,
            message_count,
            prompt_chars,
            "LLM chat 请求"
        );
        let started = Instant::now();
        let result = self
            .task_queue
            .run_tracked(TaskType::LlmCall, TaskPriority::Medium, None, {
                || async move {
                    match sidecar_call {
                        Some((client, trace)) => {
                            chat_via_sidecar(client, resolved_model, request, &trace).await
                        }
                        None => chat_embedded(resolved_model, request).await,
                    }
                }
            })
            .await;
        match &result {
            Ok(response) => {
                let response_chars = response.message.content.len();
                record_dynamic_llm_trace(
                    &self.orchestrator,
                    trace_context.as_ref(),
                    Some(model_name.as_str()),
                    response.usage.clone(),
                    Some(started.elapsed().as_millis() as i64),
                    false,
                    "completed",
                    None,
                    None,
                    response.finish_reason.as_deref(),
                )
                .await;
                tracing::debug!(
                    target: "a4n::llm",
                    model = %model_name,
                    config_id = ?config_id,
                    profile = ?profile,
                    duration_ms = started.elapsed().as_millis(),
                    finish_reason = ?response.finish_reason,
                    usage = ?response.usage,
                    response_chars,
                    "LLM chat 响应"
                );
            }
            Err(error) => {
                let message = error.to_string();
                record_dynamic_llm_trace(
                    &self.orchestrator,
                    trace_context.as_ref(),
                    Some(model_name.as_str()),
                    None,
                    Some(started.elapsed().as_millis() as i64),
                    false,
                    llm_trace_status_for_error(error),
                    Some(llm_error_kind(error)),
                    Some(message.as_str()),
                    None,
                )
                .await;
                tracing::warn!(
                    target: "a4n::llm",
                    model = %model_name,
                    config_id = ?config_id,
                    profile = ?profile,
                    duration_ms = started.elapsed().as_millis(),
                    error = %error,
                    "LLM chat 失败"
                );
            }
        }
        result
    }
}
