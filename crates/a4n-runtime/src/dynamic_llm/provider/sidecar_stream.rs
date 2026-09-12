use std::sync::Arc;

use a4n_error::AppError;
use a4n_llm::provider::ChatChunkStream;
use a4n_llm_client::LlmClient;

use super::super::{
    config::{require_sidecar_business_trace, sidecar_chat_request},
    contracts::DynamicLlmProvider,
    provider::PreparedLlmCall,
    streams::QueueTrackedSidecarLlmStream,
};

impl DynamicLlmProvider {
    pub(in crate::dynamic_llm) async fn chat_stream_via_sidecar(
        &self,
        client: Arc<dyn LlmClient>,
        prepared: PreparedLlmCall,
    ) -> Result<ChatChunkStream, AppError> {
        let PreparedLlmCall {
            model,
            request,
            trace,
            task,
            started,
        } = prepared;
        let trace =
            require_sidecar_business_trace(trace.as_ref(), "LLM sidecar stream 请求")?.clone();
        let model_name = model.model_name().to_string();
        let request = sidecar_chat_request(model, request, &trace, "LLM sidecar stream")?;
        let request_id = request.request_id.clone();
        let cancel = task.cancellation_token();
        let (tx, rx) =
            tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
        tokio::spawn(async move {
            let result = tokio::select! {
                biased;
                _ = cancel.cancelled() => return,
                result = client.stream(request, tx.clone()) => result,
            };
            if let Err(error) = result {
                let _ = tx
                    .send(a4n_service_protocol::CapabilityTaskEvent::failed(
                        request_id, 0, error,
                    ))
                    .await;
            }
        });
        Ok(Box::pin(QueueTrackedSidecarLlmStream::new(
            rx,
            self.facade.clone(),
            task,
            Some(trace),
            model_name,
            started,
        )))
    }
}
