use a4n_service_protocol::ServiceRequest;
use a4n_sidecar_bootstrap::{failed_service_event_response, service_event_ndjson_response};
use axum::{Json, response::IntoResponse};

use super::execution::image_generation_event_stream;
use crate::protocol::validate_image_request;
use a4n_image_contract::ImageProviderPayload;

pub(crate) async fn generate_events(
    Json(request): Json<ServiceRequest<ImageProviderPayload>>,
) -> impl IntoResponse {
    let task_id = request.request_id.clone();
    if let Err(error) = validate_image_request(&request) {
        return failed_service_event_response(task_id, error);
    }
    service_event_ndjson_response(image_generation_event_stream(request))
}
