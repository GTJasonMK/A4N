use crate::app::RagServiceState;
use crate::contracts::SearchPayload;
use crate::protocol::service_error;
use a4n_rag_core::vector_store::{SearchFilter, SearchResult};
use a4n_rag_sidecar_contract::{
    QueryPayload, SearchFilterPayload, SearchResultPayload, SearchTextPayload,
};
use a4n_service_protocol::{RAG_SERVICE_NAME, ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};

pub(crate) async fn query(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<QueryPayload>>,
) -> Json<ServiceResponse<Vec<SearchResultPayload>>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let result = state
        .store
        .query(
            &payload.collection,
            payload.query_embedding,
            payload.top_k,
            payload.filters.map(core_search_filter),
        )
        .await
        .map(|results| results.into_iter().map(search_result_payload).collect());
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}

fn core_search_filter(filter: SearchFilterPayload) -> SearchFilter {
    SearchFilter {
        equals: filter.equals,
        chapter_range: filter.chapter_range,
        data_types: filter.data_types,
    }
}

fn search_result_payload(result: SearchResult) -> SearchResultPayload {
    SearchResultPayload {
        id: result.id,
        content: result.content,
        score: result.score,
        metadata: result.metadata,
    }
}

pub(crate) async fn search_text(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<SearchTextPayload>>,
) -> Json<ServiceResponse<Vec<SearchResultPayload>>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let result = state
        .store
        .search_text(
            &payload.collection,
            &payload.query_text,
            payload.top_k,
            payload.filters.map(core_search_filter),
        )
        .await
        .map(|results| results.into_iter().map(search_result_payload).collect());
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}

pub(crate) async fn search(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<SearchPayload>>,
) -> Json<ServiceResponse<Vec<SearchResult>>> {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let payload = request.payload;
    let result = state
        .store
        .hybrid_search(
            &payload.collection,
            &payload.query_text,
            payload.query_embedding,
            payload.top_k,
            payload.filters,
        )
        .await;
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}
