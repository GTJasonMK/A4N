use std::sync::Arc;

use a4n_error::AppError;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult};
use a4n_video_client::VideoClient;
use tokio::sync::mpsc::UnboundedSender;

use super::super::{
    GenerationEvent, parse_generation_event, sidecar_context_for_workflow_operation_with_root,
    typed_service_request_with_context_provider_secret_policy,
};
use super::contracts::{VideoGenerationOutput, video_generation_result, video_provider_payload};
use a4n_service_protocol::{ServiceError, ServiceErrorCode};

fn video_service_error(error: ServiceError) -> AppError {
    let message = error.message;
    match error.code {
        ServiceErrorCode::Cancelled => AppError::Cancelled,
        ServiceErrorCode::ServiceUnavailable => AppError::Timeout(message),
        ServiceErrorCode::Unauthorized => AppError::Unauthorized(message),
        ServiceErrorCode::ProviderError | ServiceErrorCode::Internal => {
            AppError::Internal(anyhow::anyhow!(message))
        }
        ServiceErrorCode::CapabilityDisabled
        | ServiceErrorCode::VersionMismatch
        | ServiceErrorCode::ConfigurationError => AppError::ConfigError(message),
    }
}

pub(in crate::sidecar_clients::video) async fn generate_video_via_sidecar(
    client: Arc<dyn VideoClient>,
    config: &VideoProviderConfig,
    request: VideoGenerationRequest,
    provider_idempotency_key_sha256: Option<String>,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    progress_tx: Option<UnboundedSender<String>>,
) -> Result<VideoGenerationResult, AppError> {
    let payload =
        video_provider_payload(config, request, provider_idempotency_key_sha256, project_id)
            .map_err(AppError::ConfigError)?;
    let request = typed_service_request_with_context_provider_secret_policy(
        sidecar_context_for_workflow_operation_with_root(
            project_id,
            root_project_id,
            workflow_run_id,
            "video.generate",
        )
        .map_err(AppError::ConfigError)?,
        a4n_service_protocol::VIDEO_PROVIDER_CAPABILITY_VERSION,
        payload,
    )
    .map_err(AppError::ConfigError)?;
    let (event_tx, mut event_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
    let mut request_future = Box::pin(client.generate_events(request, event_tx));
    let mut transport_complete = false;
    let output = loop {
        tokio::select! {
            result = &mut request_future, if !transport_complete => {
                result.map_err(video_service_error)?;
                transport_complete = true;
            }
            event = event_rx.recv() => {
                let event = event.ok_or_else(|| {
                    AppError::ConfigError("video sidecar 生成事件流结束但缺少终止事件".to_string())
                })?;
                match parse_generation_event::<VideoGenerationOutput>(event, "video")
                    .map_err(AppError::ConfigError)?
                {
                    GenerationEvent::Progress(message) => {
                        if let Some(progress_tx) = progress_tx.as_ref() {
                            progress_tx
                                .send(message)
                                .map_err(|_| {
                                    AppError::ConfigError(
                                        "video sidecar progress 接收端已关闭".to_string(),
                                    )
                                })?;
                        }
                    }
                    GenerationEvent::Completed(output) => break output,
                }
            }
        }
    };
    video_generation_result(output).map_err(AppError::ConfigError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_input_contract_errors_are_not_retryable_app_errors() {
        let error = video_service_error(ServiceError::non_retryable(
            ServiceErrorCode::ConfigurationError,
            "413 Payload Too Large",
        ));
        assert!(matches!(error, AppError::ConfigError(message) if message.contains("413")));
    }

    #[test]
    fn provider_failures_keep_transient_error_semantics() {
        let error = video_service_error(ServiceError::non_retryable(
            ServiceErrorCode::ProviderError,
            "upstream unavailable",
        ));
        assert!(
            matches!(error, AppError::Internal(message) if message.to_string().contains("upstream"))
        );
    }
}
