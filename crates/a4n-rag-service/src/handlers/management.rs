use crate::app::RagServiceState;
use crate::contracts::{
    CollectionPayload, CompletenessByIdsPayload, DeleteOutput, DeletePayload, EmptyOutput,
    MetadataByFilterPayload, MetadataByIdsPayload, MetadataOutput,
};
use crate::protocol::{result_response, validate_service_request};
use a4n_rag_core::vector_store::CompletenessReport;
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};

pub(crate) async fn delete_records(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<DeletePayload>>,
) -> Json<ServiceResponse<DeleteOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let payload = request.payload.clone();
    let store = state.store.clone();
    let result = state
        .persist_mutation("delete", async move {
            store.delete(&payload.collection, payload.filter).await
        })
        .await
        .map(|deleted| DeleteOutput { deleted });
    Json(result_response(&request, result))
}

pub(crate) async fn drop_collection(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CollectionPayload>>,
) -> Json<ServiceResponse<EmptyOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let collection = request.payload.collection.clone();
    let store = state.store.clone();
    let result = state
        .persist_mutation("drop_collection", async move {
            store.drop_collection(&collection).await
        })
        .await
        .map(|_| EmptyOutput {});
    Json(result_response(&request, result))
}

pub(crate) async fn metadata_by_ids(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<MetadataByIdsPayload>>,
) -> Json<ServiceResponse<MetadataOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .metadata_by_ids(&payload.collection, &payload.ids)
        .await
        .map(|metadata| MetadataOutput { metadata });
    Json(result_response(&request, result))
}

pub(crate) async fn metadata_by_filter(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<MetadataByFilterPayload>>,
) -> Json<ServiceResponse<MetadataOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .metadata_by_filter(&payload.collection, payload.filter)
        .await
        .map(|metadata| MetadataOutput { metadata });
    Json(result_response(&request, result))
}

pub(crate) async fn completeness_report_by_ids(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CompletenessByIdsPayload>>,
) -> Json<ServiceResponse<CompletenessReport>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(request.error_response(error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .completeness_report_by_ids(&payload.collection, &payload.expected_ids)
        .await;
    Json(result_response(&request, result))
}
