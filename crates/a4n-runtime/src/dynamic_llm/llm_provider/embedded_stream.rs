use a4n_error::AppError;
use a4n_llm::provider::ChatChunkStream;

use super::super::{
    config::make_provider,
    contracts::DynamicLlmProvider,
    provider::PreparedLlmCall,
    streams::QueueTrackedLlmStream,
    trace::{llm_error_kind, llm_trace_status_for_error, record_dynamic_llm_trace},
};

impl DynamicLlmProvider {
    pub(super) async fn chat_stream_via_embedded(
        &self,
        prepared: PreparedLlmCall,
    ) -> Result<ChatChunkStream, AppError> {
        let PreparedLlmCall {
            model,
            request,
            trace,
            task,
            started,
        } = prepared;
        let provider = make_provider(&model.model)?;
        let model_name = model.model_name().to_string();
        let cancel = task.cancellation_token();
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(AppError::Cancelled),
            stream = provider.chat_stream(request) => stream,
        };
        match result {
            Ok(stream) => Ok(Box::pin(QueueTrackedLlmStream::new(
                stream,
                self.facade.clone(),
                task,
                trace,
                model_name,
                started,
            ))),
            Err(error) => {
                let status = if matches!(error, AppError::Cancelled) {
                    task.cancel().await
                } else {
                    task.fail(error.to_string()).await
                };
                let error = match status {
                    Ok(()) => error,
                    Err(status_error) => AppError::Internal(anyhow::anyhow!(
                        "{error}；同步 LLM stream 建立失败状态失败: {status_error}"
                    )),
                };
                record_dynamic_llm_trace(
                    &self.facade,
                    trace.as_ref(),
                    Some(&model_name),
                    None,
                    Some(started.elapsed().as_millis() as i64),
                    true,
                    llm_trace_status_for_error(&error),
                    Some(llm_error_kind(&error)),
                    Some(&error.to_string()),
                    None,
                )
                .await;
                Err(error)
            }
        }
    }
}
