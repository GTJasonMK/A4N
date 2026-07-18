use crate::app::RagServiceState;
use crate::contracts::{
    BoolOutput, CollectionPayload, CollectionsOutput, CompletenessByIdsPayload,
    CompletenessPayload, CountOutput, DeleteOutput, DeletePayload, EmptyOutput,
    MetadataByFilterPayload, MetadataByIdsPayload, MetadataOutput, UpdateMetadataPayload,
};
use crate::protocol::{error_response, result_response, validate_service_request};
use a4n_rag_core::vector_store::CompletenessReport;
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};

pub(crate) async fn delete_records(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<DeletePayload>>,
) -> Json<ServiceResponse<DeleteOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .delete(&payload.collection, payload.filter)
        .await
        .map(|deleted| DeleteOutput { deleted });
    Json(result_response(&request, result))
}

pub(crate) async fn collection_exists(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CollectionPayload>>,
) -> Json<ServiceResponse<BoolOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .store
        .collection_exists(&request.payload.collection)
        .await
        .map(|value| BoolOutput { value });
    Json(result_response(&request, result))
}

pub(crate) async fn count_records(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CollectionPayload>>,
) -> Json<ServiceResponse<CountOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .store
        .count(&request.payload.collection)
        .await
        .map(|count| CountOutput { count });
    Json(result_response(&request, result))
}

pub(crate) async fn drop_collection(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CollectionPayload>>,
) -> Json<ServiceResponse<EmptyOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .store
        .drop_collection(&request.payload.collection)
        .await
        .map(|_| EmptyOutput {});
    Json(result_response(&request, result))
}

pub(crate) async fn list_collections(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<serde_json::Value>>,
) -> Json<ServiceResponse<CollectionsOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .store
        .list_collections()
        .await
        .map(|collections| CollectionsOutput { collections });
    Json(result_response(&request, result))
}

pub(crate) async fn update_metadata(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<UpdateMetadataPayload>>,
) -> Json<ServiceResponse<EmptyOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .update_metadata(&payload.collection, &payload.id, payload.metadata)
        .await
        .map(|_| EmptyOutput {});
    Json(result_response(&request, result))
}

pub(crate) async fn metadata_by_ids(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<MetadataByIdsPayload>>,
) -> Json<ServiceResponse<MetadataOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
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
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .metadata_by_filter(&payload.collection, payload.filter)
        .await
        .map(|metadata| MetadataOutput { metadata });
    Json(result_response(&request, result))
}

pub(crate) async fn health_check(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<serde_json::Value>>,
) -> Json<ServiceResponse<BoolOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = state
        .store
        .health_check()
        .await
        .map(|value| BoolOutput { value });
    Json(result_response(&request, result))
}

pub(crate) async fn completeness_report(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CompletenessPayload>>,
) -> Json<ServiceResponse<CompletenessReport>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .completeness_report(&payload.collection, payload.expected_count)
        .await;
    Json(result_response(&request, result))
}

pub(crate) async fn completeness_report_by_ids(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CompletenessByIdsPayload>>,
) -> Json<ServiceResponse<CompletenessReport>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .completeness_report_by_ids(&payload.collection, &payload.expected_ids)
        .await;
    Json(result_response(&request, result))
}
