use std::{pin::Pin, time::Instant};

use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest};
use a4n_queue::priority::{TaskKind, TaskPriority};
use futures::Stream;

use super::super::{
    DynamicLlmProvider, QueueTrackedLlmStream, ensure_max_tokens_within_model_cap, make_provider,
    trace::{
        llm_error_kind, llm_trace_status_for_error, prepare_dynamic_llm_trace_context,
        record_dynamic_llm_trace,
    },
};

impl DynamicLlmProvider {
    pub(super) async fn chat_stream_via_embedded(
        &self,
        mut request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        let profile = request.max_tokens_profile.clone();
        let temperature = request.temperature;
        let message_count = request.messages.len();
        let prompt_chars: usize = request.messages.iter().map(|msg| msg.content.len()).sum();
        let resolved_model = self.resolve_runtime_model_by_id(None).await?;
        let provider = make_provider(&resolved_model.model)?;
        let model_name = provider.model_name().to_string();
        let model_max_output_tokens = provider.capabilities().max_output_tokens;
        request.max_tokens = self.resolve_max_tokens(&request).await?;
        ensure_max_tokens_within_model_cap(
            request.max_tokens,
            profile.as_deref(),
            model_max_output_tokens,
            model_name.as_str(),
        )?;
        let trace_context =
            prepare_dynamic_llm_trace_context(&self.facade, provider.as_ref(), &mut request)
                .await?;
        if trace_context.is_some() {
            request.stream_include_usage = true;
        }
        tracing::debug!(
            target: "a4n::llm",
            model = %model_name,
            profile = ?profile,
            max_tokens = ?request.max_tokens,
            temperature = ?temperature,
            message_count,
            prompt_chars,
            "LLM stream 请求"
        );
        let started = Instant::now();
        let submitted_id = self
            .facade
            .queue_submit(TaskKind::LlmCall, TaskPriority::Medium, None)
            .await?;
        let mut submitted_guard = self.facade.queue_submitted_task_guard(submitted_id.clone());
        let Some(permit) = self
            .facade
            .queue_acquire_task_with_cancel(
                &submitted_id,
                tokio_util::sync::CancellationToken::new(),
            )
            .await?
        else {
            let cancel_result = self.facade.queue_cancel(&submitted_id).await;
            submitted_guard.disarm();
            return match cancel_result {
                Ok(()) => Err(AppError::Cancelled),
                Err(status_error) => Err(AppError::Internal(anyhow::anyhow!(
                    "LLM stream 任务已取消；同步队列取消状态失败: {status_error}"
                ))),
            };
        };
        submitted_guard.disarm();
        let running_id = permit.task_id().to_string();
        let result = provider.chat_stream(request).await;
        match result {
            Ok(stream) => Ok(Box::pin(QueueTrackedLlmStream::new(
                stream,
                self.facade.clone(),
                permit,
                trace_context,
                model_name,
                started,
            ))),
            Err(error) => {
                let message = error.to_string();
                permit.fail(message.clone());
                let error = match self.facade.queue_mark_failed(&running_id, &message).await {
                    Ok(()) => error,
                    Err(status_error) => AppError::Internal(anyhow::anyhow!(
                        "{error}；同步 LLM stream 建立失败队列状态失败: {status_error}"
                    )),
                };
                let message = error.to_string();
                record_dynamic_llm_trace(
                    &self.facade,
                    trace_context.as_ref(),
                    Some(model_name.as_str()),
                    None,
                    Some(started.elapsed().as_millis() as i64),
                    true,
                    llm_trace_status_for_error(&error),
                    Some(llm_error_kind(&error)),
                    Some(&message),
                    None,
                )
                .await;
                tracing::warn!(
                    target: "a4n::llm",
                    model = %model_name,
                    profile = ?profile,
                    duration_ms = started.elapsed().as_millis(),
                    error = %error,
                    "LLM stream 建立失败"
                );
                Err(error)
            }
        }
    }
}
