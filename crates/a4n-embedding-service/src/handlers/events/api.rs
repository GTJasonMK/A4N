use a4n_service_protocol::ServiceRequest;
use a4n_sidecar_bootstrap::{service_event_ndjson_response, single_service_event_stream};
use axum::{Json, response::IntoResponse};

use super::execution::embedding_download_event_stream;
use super::projection::failed_download_event;
use crate::contracts::ModelPayload;
use crate::handlers::models::resolve_cache_dir;
use crate::protocol::{service_error, validate_service_request};

pub(crate) async fn download_model_events(
    Json(request): Json<ServiceRequest<ModelPayload>>,
) -> impl IntoResponse {
    let task_id = request.request_id.clone();
    if let Err(error) = validate_service_request(&request) {
        return failed_response(task_id, error);
    }
    let model = match request.payload.required_model() {
        Ok(model) => model.to_string(),
        Err(error) => return failed_response(task_id, service_error(&error)),
    };
    let cache_dir = match resolve_cache_dir() {
        Ok(cache_dir) => cache_dir,
        Err(error) => return failed_response(task_id, service_error(&error)),
    };
    service_event_ndjson_response(
        embedding_download_event_stream(task_id, model, cache_dir),
        "embedding download",
    )
}

fn failed_response(
    task_id: String,
    error: a4n_service_protocol::ServiceError,
) -> axum::response::Response {
    service_event_ndjson_response(
        single_service_event_stream(failed_download_event(task_id, 1, error)),
        "embedding download",
    )
}
