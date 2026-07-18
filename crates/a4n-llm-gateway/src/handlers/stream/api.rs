use a4n_service_protocol::{ServiceError, ServiceRequest};
use a4n_sidecar_bootstrap::{service_event_ndjson_response, single_service_event_stream};
use axum::{Json, response::Response};

use super::execution::{EventLineStream, stream_with_provider};
use super::projection::failed_event;
use crate::contracts::LlmChatPayload;
use crate::protocol::{service_error, validate_provider_secret_policy, validate_service_request};

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

    let task_id = request.request_id.clone();
    let events = match stream_with_provider(request).await {
        Ok(events) => events,
        Err(error) => Box::pin(single_service_event_stream(failed_event(
            task_id,
            1,
            service_error(error),
        ))) as EventLineStream,
    };

    service_event_ndjson_response(events, "LLM stream")
}

fn failed_stream_response(task_id: String, error: ServiceError) -> Response {
    service_event_ndjson_response(
        single_service_event_stream(failed_event(task_id, 1, error)),
        "LLM stream",
    )
}
