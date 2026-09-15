use a4n_llm_sidecar_contract::{LlmChatRequest, LlmGatewayPayload};
use a4n_service_protocol::{LLM_SERVICE_NAME, ServiceRequest};
use a4n_sidecar_bootstrap::{failed_service_event_response, service_event_ndjson_response};
use axum::{Json, response::Response};

use super::execution::stream_with_provider;

pub(crate) async fn stream_chat(
    Json(request): Json<ServiceRequest<LlmGatewayPayload<LlmChatRequest>>>,
) -> Response {
    if let Err(error) = request.validate_known_provider_request(
        LLM_SERVICE_NAME,
        request.payload.provider.api_key.as_deref(),
    ) {
        return failed_service_event_response(request.request_id, error);
    }

    service_event_ndjson_response(stream_with_provider(request))
}
