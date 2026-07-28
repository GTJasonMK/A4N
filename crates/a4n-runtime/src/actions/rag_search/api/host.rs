use super::execution::{execute_enhanced_rag_search, execute_enhanced_rag_search_with_vectors};
use crate::{AppState, actions::access};
use a4n_core::error::AppError;
use a4n_orchestrator::rag::{normalize_search_data_types, required_nonblank, required_top_k};

use super::super::{
    EnhancedSearchInput, RagEnhancedSearchOutput, RagEnhancedSearchWithVectorsInput,
    RagPreparedEnhancedSearchOutput, preparation::prepare_enhanced_rag_search,
};

pub async fn rag_enhanced_search(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let query = required_nonblank(&input.query, "rag_enhanced_search query")?;
    let top_k = required_top_k(input.top_k, "rag_enhanced_search")?;
    let data_types = normalize_search_data_types(input.data_types)?;
    let rag = state.rag();
    execute_enhanced_rag_search(
        &rag,
        project_id,
        &query,
        top_k,
        input.chapter_number,
        data_types,
    )
    .await
}

pub async fn rag_prepare_enhanced_search(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<RagPreparedEnhancedSearchOutput, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let rag = state.rag();
    let prepared = prepare_enhanced_rag_search(&rag, project_id, input).await?;
    Ok(RagPreparedEnhancedSearchOutput {
        queries: prepared
            .queries
            .iter()
            .map(super::super::rag_enhanced_query_summary)
            .collect(),
        query_texts: prepared
            .queries
            .iter()
            .map(|query| query.query.clone())
            .collect(),
        embedding_identity: super::super::provider_identity_to_contract(
            prepared.embedding_identity,
        ),
        top_k: prepared.top_k,
        pull_k: prepared.pull_k,
    })
}

pub async fn rag_enhanced_search_with_vectors(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: RagEnhancedSearchWithVectorsInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let rag = state.rag();
    execute_enhanced_rag_search_with_vectors(&rag, project_id, input).await
}
