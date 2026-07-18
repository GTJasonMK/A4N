use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, LlmProvider};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;

use super::provider_from_config;
use crate::contracts::{LlmProbeOutput, LlmProbePayload};
use crate::protocol::{
    error_response, result_response, validate_provider_secret_policy, validate_service_request,
};

pub(crate) async fn probe(
    Json(request): Json<ServiceRequest<LlmProbePayload>>,
) -> Json<ServiceResponse<LlmProbeOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    if let Err(error) = validate_provider_secret_policy(
        request.payload.provider.api_key.as_deref(),
        &request.secret_policy,
    ) {
        return Json(error_response(&request, error));
    }
    let result = probe_with_provider(request.payload.clone()).await;
    Json(result_response(&request, result))
}

async fn probe_with_provider(payload: LlmProbePayload) -> Result<LlmProbeOutput, AppError> {
    let provider = provider_from_config(payload.provider)?;
    probe_existing_provider(&provider, payload.request).await
}

pub(crate) async fn probe_existing_provider(
    provider: &dyn LlmProvider,
    request: Option<ChatRequest>,
) -> Result<LlmProbeOutput, AppError> {
    match request {
        Some(request) => {
            let response = provider.chat(request).await?;
            let preview = response
                .message
                .content
                .chars()
                .take(200)
                .collect::<String>();
            Ok(LlmProbeOutput {
                ok: !response.message.content.trim().is_empty(),
                response_preview: (!preview.is_empty()).then_some(preview),
                usage: response.usage,
                finish_reason: response.finish_reason,
            })
        }
        None => provider.test_connection().await.map(|ok| LlmProbeOutput {
            ok,
            response_preview: None,
            usage: None,
            finish_reason: None,
        }),
    }
}
