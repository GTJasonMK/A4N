use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_rag_core::{
    PreparedEnhancedRagSearch,
    search::{normalize_search_data_types, required_nonblank, required_top_k},
};

use super::EnhancedSearchInput;
use crate::{actions::access, state::AppState};

pub(super) async fn prepare_enhanced_rag_search(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<PreparedEnhancedRagSearch, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let rag = state.rag_search();
    let query = required_nonblank(&input.query)?;
    let top_k = required_top_k(input.top_k)?;
    let data_types = normalize_search_data_types(input.data_types)?;
    let chapter_number = match input.chapter_number {
        Some(value) if value <= 0 => {
            return Err(AppError::InvalidArgument(
                "rag_enhanced_search chapter_number 必须大于 0".to_string(),
            ));
        }
        value => value,
    };
    let queries = rag
        .prepare_enhanced_rag_queries(project_id, &query, chapter_number)
        .await?;
    let embedding_identity = current_embedding_identity(state).await?;
    let filter = rag
        .enhanced_rag_search_filter(project_id, &embedding_identity, data_types)
        .await?;
    Ok(PreparedEnhancedRagSearch {
        project_id,
        queries,
        filter,
        top_k,
        chapter_number,
        embedding_identity,
    })
}

pub(crate) async fn current_embedding_identity(
    state: &crate::AppState,
) -> Result<EmbeddingModelIdentity, AppError> {
    if let Some(model) = crate::runtime_config::current_embedding_provider_config() {
        return crate::dynamic_embedding::model_identity_from_config(&model);
    }
    state.rag_search().embedding_model_identity().await
}
