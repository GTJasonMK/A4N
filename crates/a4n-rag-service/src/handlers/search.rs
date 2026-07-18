use crate::app::RagServiceState;
use crate::contracts::{QueryPayload, SearchPayload, SearchTextPayload};
use crate::protocol::{error_response, result_response, validate_service_request};
use a4n_rag_core::vector_store::SearchResult;
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::{Json, extract::State};

pub(crate) async fn query(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<QueryPayload>>,
) -> Json<ServiceResponse<Vec<SearchResult>>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .query(
            &payload.collection,
            payload.query_embedding,
            payload.top_k,
            payload.filters,
        )
        .await;
    Json(result_response(&request, result))
}

pub(crate) async fn search_text(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<SearchTextPayload>>,
) -> Json<ServiceResponse<Vec<SearchResult>>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .search_text(
            &payload.collection,
            &payload.query_text,
            payload.top_k,
            payload.filters,
        )
        .await;
    Json(result_response(&request, result))
}

pub(crate) async fn search(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<SearchPayload>>,
) -> Json<ServiceResponse<Vec<SearchResult>>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let payload = request.payload.clone();
    let result = state
        .store
        .hybrid_search(
            &payload.collection,
            &payload.query_text,
            payload.query_embedding,
            payload.top_k,
            payload.filters,
            payload.vector_weight,
        )
        .await;
    Json(result_response(&request, result))
}
