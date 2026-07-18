use crate::AppState;
use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_orchestrator::rag::{
    PreparedEnhancedRagSearch, enhanced_rag_pull_k, normalize_search_data_types, required_nonblank,
    required_top_k,
};

use super::EnhancedSearchInput;

pub(super) async fn prepare_enhanced_rag_search(
    state: &AppState,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<PreparedEnhancedRagSearch, AppError> {
    let query = required_nonblank(&input.query, "rag_enhanced_search query")?;
    let top_k = required_top_k(input.top_k, "rag_enhanced_search")?;
    let data_types = normalize_search_data_types(input.data_types)?;
    prepare_enhanced_rag_search_parts(
        state,
        project_id,
        &query,
        top_k,
        input.chapter_number,
        data_types,
    )
    .await
}

pub(super) async fn prepare_enhanced_rag_search_parts(
    state: &AppState,
    project_id: i64,
    base_query: &str,
    top_k: usize,
    chapter_number: Option<i32>,
    data_types: Option<Vec<String>>,
) -> Result<PreparedEnhancedRagSearch, AppError> {
    let queries = state
        .orchestrator
        .prepare_enhanced_rag_queries(project_id, base_query, chapter_number)
        .await?;
    let embedding_identity = current_embedding_identity(state).await?;
    let filter = state
        .orchestrator
        .enhanced_rag_search_filter(project_id, &embedding_identity, data_types)
        .await?;
    Ok(PreparedEnhancedRagSearch {
        queries,
        filter,
        top_k,
        pull_k: enhanced_rag_pull_k(top_k),
        chapter_number,
        embedding_identity,
    })
}

pub(crate) async fn current_embedding_identity(
    state: &AppState,
) -> Result<EmbeddingModelIdentity, AppError> {
    if let Some(model) = crate::runtime_config::current_embedding_model() {
        return crate::dynamic_embedding::model_identity_from_config(&model);
    }
    state.embedding.model_identity().await
}
