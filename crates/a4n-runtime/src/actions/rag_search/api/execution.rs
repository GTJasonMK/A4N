use super::output::project_enhanced_rag_search_output;
use a4n_core::error::AppError;

use super::super::{
    RagEnhancedSearchOutput, RagEnhancedSearchWithVectorsInput,
    preparation::prepare_enhanced_rag_search,
};

pub(super) async fn execute_enhanced_rag_search_with_vectors(
    rag: &a4n_orchestrator::OrchestratorContext,
    project_id: i64,
    input: RagEnhancedSearchWithVectorsInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search(rag, project_id, input.search).await?;
    let result = rag
        .execute_prepared_enhanced_rag_search_with_vectors(
            project_id,
            &prepared,
            &crate::dynamic_embedding::contract_identity_to_provider(&input.embedding_identity),
            input.query_vectors,
        )
        .await?;
    Ok(project_enhanced_rag_search_output(result))
}

pub(super) async fn execute_enhanced_rag_search(
    rag: &a4n_orchestrator::OrchestratorContext,
    project_id: i64,
    input: super::super::EnhancedSearchInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search(rag, project_id, input).await?;
    let result = rag
        .execute_prepared_enhanced_rag_search(project_id, &prepared)
        .await?;
    Ok(project_enhanced_rag_search_output(result))
}
