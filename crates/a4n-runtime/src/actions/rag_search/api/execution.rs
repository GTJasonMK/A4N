use super::output::project_enhanced_rag_search_output;
use crate::AppState;
use a4n_core::error::AppError;

use super::super::{
    RagEnhancedSearchOutput, RagEnhancedSearchWithVectorsInput,
    preparation::{prepare_enhanced_rag_search, prepare_enhanced_rag_search_parts},
};

pub(super) async fn execute_enhanced_rag_search_with_vectors(
    state: &AppState,
    project_id: i64,
    input: RagEnhancedSearchWithVectorsInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search(state, project_id, input.search).await?;
    let result = state
        .orchestrator
        .execute_prepared_enhanced_rag_search_with_vectors(
            project_id,
            &prepared,
            &input.embedding_identity,
            input.query_vectors,
        )
        .await?;
    Ok(project_enhanced_rag_search_output(result))
}

pub(super) async fn execute_enhanced_rag_search(
    state: &AppState,
    project_id: i64,
    base_query: &str,
    top_k: usize,
    chapter_number: Option<i32>,
    data_types: Option<Vec<String>>,
) -> Result<RagEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search_parts(
        state,
        project_id,
        base_query,
        top_k,
        chapter_number,
        data_types,
    )
    .await?;
    let result = state
        .orchestrator
        .execute_prepared_enhanced_rag_search(project_id, &prepared)
        .await?;
    Ok(project_enhanced_rag_search_output(result))
}
