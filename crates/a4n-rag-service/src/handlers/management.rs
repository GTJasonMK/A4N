use std::collections::HashMap;

use crate::app::RagServiceState;
use crate::contracts::{
    CollectionPayload, CompletenessByIdsPayload, DeletePayload, MetadataByFilterPayload,
    MetadataByIdsPayload,
};
use crate::protocol::service_error;
use a4n_rag_core::vector_store::CompletenessReport;
use a4n_service_protocol::{RAG_SERVICE_NAME, ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};
use serde_json::Value;

pub(crate) async fn delete_records(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<DeletePayload>>,
) -> Json<ServiceResponse<usize>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let store = state.store.clone();
    let result = state
        .persist_mutation(async move {
            store.delete(&payload.collection, payload.filter).await
        })
        .await;
    Json(ServiceResponse::from_result(trace_id, result, service_error))
}

pub(crate) async fn drop_collection(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CollectionPayload>>,
) -> Json<ServiceResponse<()>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let collection = request.payload.collection;
    let store = state.store.clone();
    let result = state
        .persist_mutation(async move {
            store.drop_collection(&collection).await
        })
        .await;
    Json(ServiceResponse::from_result(trace_id, result, service_error))
}

pub(crate) async fn metadata_by_ids(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<MetadataByIdsPayload>>,
) -> Json<ServiceResponse<HashMap<String, Value>>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let result = state
        .store
        .metadata_by_ids(&payload.collection, &payload.ids)
        .await;
    Json(ServiceResponse::from_result(trace_id, result, service_error))
}

pub(crate) async fn metadata_by_filter(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<MetadataByFilterPayload>>,
) -> Json<ServiceResponse<HashMap<String, Value>>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let result = state
        .store
        .metadata_by_filter(&payload.collection, payload.filter)
        .await;
    Json(ServiceResponse::from_result(trace_id, result, service_error))
}

pub(crate) async fn completeness_report_by_ids(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CompletenessByIdsPayload>>,
) -> Json<ServiceResponse<CompletenessReport>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let result = state
        .store
        .completeness_report_by_ids(&payload.collection, &payload.expected_ids)
        .await;
    Json(ServiceResponse::from_result(trace_id, result, service_error))
}
