use a4n_embedding_contract::EmbeddingModelPayload;
use a4n_service_protocol::{EMBEDDING_SERVICE_NAME, ServiceRequest};
use a4n_sidecar_bootstrap::{failed_service_event_response, service_event_ndjson_response};
use axum::{Json, response::IntoResponse};

use super::execution::embedding_download_event_stream;
use crate::handlers::models::{required_model, resolve_cache_dir};
use crate::protocol::service_error;

pub(crate) async fn download_model_events(
    Json(request): Json<ServiceRequest<EmbeddingModelPayload>>,
) -> impl IntoResponse {
    if let Err(error) = request.validate_known_service(EMBEDDING_SERVICE_NAME) {
        return failed_service_event_response(request.request_id, error);
    }
    let task_id = request.request_id;
    let model = match required_model(request.payload) {
        Ok(model) => model,
        Err(error) => return failed_service_event_response(task_id, service_error(error)),
    };
    let cache_dir = match resolve_cache_dir() {
        Ok(cache_dir) => cache_dir,
        Err(error) => return failed_service_event_response(task_id, service_error(error)),
    };
    service_event_ndjson_response(embedding_download_event_stream(task_id, model, cache_dir))
}
