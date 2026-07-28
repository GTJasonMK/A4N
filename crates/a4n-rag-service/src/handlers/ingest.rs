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
    let payload = request.payload.clone();
    let store = state.store.clone();
    let result = state
        .persist_mutation("ingest", async move {
            store
                .add_embeddings(&payload.collection, payload.records)
                .await
        })
        .await
        .map(|written| IngestOutput { written });
    Json(result_response(&request, result))
}
