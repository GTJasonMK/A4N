use super::output::project_enhanced_rag_search_output;
use crate::state::RagRuntimeFacade;
use a4n_core::error::AppError;

use super::super::contract_identity_to_provider;
use super::super::{
    RagEnhancedSearchOutput, RagEnhancedSearchWithVectorsInput,
    preparation::{prepare_enhanced_rag_search, prepare_enhanced_rag_search_parts},
};

pub(super) async fn execute_enhanced_rag_search_with_vectors(
    rag: &RagRuntimeFacade<'_>,
    project_id: i64,
    input: RagEnhancedSearchWithVectorsInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search(rag, project_id, input.search).await?;
    let result = rag
        .execute_prepared_enhanced_rag_search_with_vectors(
            project_id,
            &prepared,
            &contract_identity_to_provider(&input.embedding_identity),
            input.query_vectors,
        )
        .await?;
    Ok(project_enhanced_rag_search_output(result))
}

pub(super) async fn execute_enhanced_rag_search(
    rag: &RagRuntimeFacade<'_>,
    project_id: i64,
    base_query: &str,
    top_k: usize,
    chapter_number: Option<i32>,
    data_types: Option<Vec<String>>,
) -> Result<RagEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search_parts(
        rag,
        project_id,
        base_query,
        top_k,
        chapter_number,
        data_types,
    )
    .await?;
    let result = rag
        .execute_prepared_enhanced_rag_search(project_id, &prepared)
        .await?;
    Ok(project_enhanced_rag_search_output(result))
}
