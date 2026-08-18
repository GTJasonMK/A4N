use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use a4n_capabilities::LlmClient;
use a4n_core::error::AppError;
use a4n_llm::provider::{ChatChunk, ChatRequest};
use a4n_queue::priority::{TaskKind, TaskPriority};
use futures::Stream;

use super::super::budget::ensure_max_tokens_within_model_cap;
use super::super::config::{require_sidecar_business_trace, sidecar_chat_request};
use super::super::contracts::DynamicLlmProvider;
use super::super::streams::QueueTrackedSidecarLlmStream;
use super::super::trace::prepare_dynamic_llm_trace_context;

impl DynamicLlmProvider {
    pub(in crate::dynamic_llm) async fn chat_stream_via_sidecar(
        &self,
        client: Arc<dyn LlmClient>,
        mut request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
        let profile = request.max_tokens_profile.clone();
        let temperature = request.temperature;
        let message_count = request.messages.len();
        let prompt_chars: usize = request.messages.iter().map(|msg| msg.content.len()).sum();
        let resolved_model = self.resolve_runtime_model_by_id(None).await?;
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
            prepare_dynamic_llm_trace_context(&self.facade, &planner, &mut request).await?;
        let trace_context =
            require_sidecar_business_trace(trace_context.as_ref(), "LLM sidecar stream 请求")?
                .clone();
        tracing::debug!(
            target: "a4n::llm",
            model = %model_name,
            profile = ?profile,
            max_tokens = ?request.max_tokens,
            temperature = ?temperature,
            message_count,
            prompt_chars,
            "LLM sidecar stream 请求"
        );

        let sidecar_request =
            sidecar_chat_request(resolved_model, request, &trace_context, "LLM sidecar stream")?;
        let sidecar_request_id = sidecar_request.request_id.clone();
        let started = Instant::now();
        let running_task = self
            .facade
            .queue_start_task(TaskKind::LlmCall, TaskPriority::Medium, None)
            .await?;

        let (tx, rx) =
            tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
        let stream_tx = tx.clone();
        tokio::spawn(async move {
            if let Err(error) = client.stream(sidecar_request, stream_tx.clone()).await {
                let _ = stream_tx
                    .send(a4n_service_protocol::CapabilityTaskEvent::failed(
                        sidecar_request_id,
                        0,
                        error,
                    ))
                    .await;
            }
        });

        Ok(Box::pin(QueueTrackedSidecarLlmStream::new(
            rx,
            self.facade.clone(),
            running_task,
            Some(trace_context),
            model_name,
            started,
        )))
    }
}
