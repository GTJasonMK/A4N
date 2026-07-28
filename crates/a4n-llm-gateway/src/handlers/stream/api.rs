use a4n_service_protocol::{ServiceError, ServiceRequest};
use a4n_sidecar_bootstrap::{service_event_ndjson_response, single_service_event_stream};
use axum::{Json, response::Response};

use super::execution::stream_with_provider;
use super::projection::failed_event;
use crate::contracts::LlmChatPayload;
use crate::protocol::{validate_provider_secret_policy, validate_service_request};

pub(crate) async fn stream_chat(Json(request): Json<ServiceRequest<LlmChatPayload>>) -> Response {
    if let Err(error) = validate_service_request(&request) {
        return failed_stream_response(request.request_id, error);
    }
    if let Err(error) = validate_provider_secret_policy(
        request.payload.provider.api_key.as_deref(),
        &request.secret_policy,
    ) {
        return failed_stream_response(request.request_id, error);
    }

    service_event_ndjson_response(stream_with_provider(request), "LLM stream")
}

fn failed_stream_response(task_id: String, error: ServiceError) -> Response {
    service_event_ndjson_response(
        single_service_event_stream(failed_event(task_id, 1, error)),
        "LLM stream",
    )
}
