use crate::app::RagServiceState;
use crate::contracts::{IngestOutput, IngestPayload};
use crate::protocol::{error_response, result_response, validate_service_request};
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};

pub(crate) async fn ingest(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<IngestPayload>>,
) -> Json<ServiceResponse<IngestOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .store
        .add_embeddings(&request.payload.collection, request.payload.records.clone())
        .await
        .map(|written| IngestOutput { written });
    Json(result_response(&request, result))
}
