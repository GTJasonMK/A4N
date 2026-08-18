use super::output::project_enhanced_rag_search_output;
use crate::AppState;
use a4n_core::error::AppError;

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
    let prepared = prepare_enhanced_rag_search(state, user_id, project_id, input).await?;
    let result = state
        .orchestrator()
        .execute_prepared_enhanced_rag_search(prepared)
        .await?;
    Ok(project_enhanced_rag_search_output(result))
}

pub async fn rag_prepare_enhanced_search(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<RagPreparedEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search(state, user_id, project_id, input).await?;
    Ok(RagPreparedEnhancedSearchOutput {
        query_texts: prepared
            .queries
            .into_iter()
            .map(|query| query.query)
            .collect(),
        embedding_identity: prepared.embedding_identity,
    })
}

pub async fn rag_enhanced_search_with_vectors(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: RagEnhancedSearchWithVectorsInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search(state, user_id, project_id, input.search).await?;
    let result = state
        .orchestrator()
        .execute_prepared_enhanced_rag_search_with_vectors(
            prepared,
            &input.embedding_identity,
            input.query_vectors,
        )
        .await?;
    Ok(project_enhanced_rag_search_output(result))
}
