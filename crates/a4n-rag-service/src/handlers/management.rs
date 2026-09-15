use std::collections::HashMap;

use crate::app::RagServiceState;
use crate::protocol::service_error;
use a4n_rag_core::vector_store::{CompletenessReport, DeleteFilter};
use a4n_rag_sidecar_contract::{
    CompletenessByIdsPayload, CompletenessReport as CompletenessReportPayload, DeleteFilterPayload,
    DeleteRecordsPayload, DropCollectionPayload, MetadataByFilterPayload, MetadataByIdsPayload,
};
use a4n_service_protocol::{RAG_SERVICE_NAME, ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};
use serde_json::Value;

pub(crate) async fn delete_records(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<DeleteRecordsPayload>>,
) -> Json<ServiceResponse<usize>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let filter = core_delete_filter(payload.filter);
    let collection = payload.collection;
    let store = state.store.clone();
    let result = state
        .persist_mutation(async move { store.delete(&collection, filter).await })
        .await;
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}

pub(crate) async fn drop_collection(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<DropCollectionPayload>>,
) -> Json<ServiceResponse<()>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let collection = request.payload.collection;
    let store = state.store.clone();
    let result = state
        .persist_mutation(async move { store.drop_collection(&collection).await })
        .await;
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
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
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
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
    let filter = core_delete_filter(payload.filter);
    let collection = payload.collection;
    let result = state.store.metadata_by_filter(&collection, filter).await;
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}

fn core_delete_filter(filter: DeleteFilterPayload) -> DeleteFilter {
    DeleteFilter {
        ids: filter.ids,
        id_prefixes: filter.id_prefixes,
    }
}

pub(crate) async fn completeness_report_by_ids(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<CompletenessByIdsPayload>>,
) -> Json<ServiceResponse<CompletenessReportPayload>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let result = state
        .store
        .completeness_report_by_ids(&payload.collection, &payload.expected_ids)
        .await
        .map(|report: CompletenessReport| CompletenessReportPayload {
            collection: report.collection,
            expected: report.expected,
            actual_vector: report.actual_vector,
            actual_bm25: report.actual_bm25,
            is_complete: report.is_complete,
            missing_vector_ids: report.missing_vector_ids,
            missing_bm25_ids: report.missing_bm25_ids,
            missing_ids: report.missing_ids,
        });
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}
