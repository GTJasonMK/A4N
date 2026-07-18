use a4n_service_protocol::{ServiceError, ServiceRequest};
use a4n_sidecar_bootstrap::{service_event_ndjson_response, single_service_event_stream};
use axum::{Json, response::IntoResponse};

use super::execution::image_generation_event_stream;
use super::projection::failed_generation_event;
use crate::contracts::ImageProviderPayload;
use crate::protocol::validate_image_request;

pub(crate) async fn generate_events(
    Json(request): Json<ServiceRequest<ImageProviderPayload>>,
) -> impl IntoResponse {
    let task_id = request.request_id.clone();
    if let Err(error) = validate_image_request(&request) {
        return failed_response(task_id, error);
    }
    service_event_ndjson_response(image_generation_event_stream(request), "image generation")
}

fn failed_response(task_id: String, error: ServiceError) -> axum::response::Response {
    service_event_ndjson_response(
        single_service_event_stream(failed_generation_event(task_id, 1, error)),
        "image generation",
    )
}
