use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatChunkStream;

use super::super::{
    config::make_provider,
    contracts::DynamicLlmProvider,
    provider::PreparedLlmCall,
    streams::QueueTrackedLlmStream,
    trace::{
        DynamicLlmTraceOutcome, llm_error_kind, llm_trace_status_for_error,
        record_dynamic_llm_trace,
    },
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
            cost_record,
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
                cost_record,
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
                let _ = self
                    .facade
                    .finish_model_usage(
                        cost_record,
                        a4n_domain::cost::ModelUsageRecordCompletion {
                            status: "failed".to_string(),
                            meter: Default::default(),
                            error_kind: Some("provider".to_string()),
                            error_message: Some(error.to_string()),
                        },
                    )
                    .await;
                let error_message = error.to_string();
                record_dynamic_llm_trace(
                    &self.facade,
                    trace.as_ref(),
                    DynamicLlmTraceOutcome {
                        model: &model_name,
                        usage: None,
                        duration_ms: Some(started.elapsed().as_millis() as i64),
                        status: llm_trace_status_for_error(&error),
                        error_kind: Some(llm_error_kind(&error)),
                        error_message: Some(&error_message),
                        finish_reason: None,
                    },
                )
                .await;
                Err(error)
            }
        }
    }
}
