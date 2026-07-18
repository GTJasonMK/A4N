use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingModelIdentity, EmbeddingProvider};
use a4n_rag_core::vector_store::{SearchFilter, SearchResult};

use super::execution::{
    hybrid_rag_search_filtered_in_store, hybrid_rag_search_with_embedding_in_store,
};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn hybrid_rag_search_filtered(
        &self,
        project_id: i64,
        query: &str,
        provider: &dyn EmbeddingProvider,
        top_k: usize,
        filter: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        hybrid_rag_search_filtered_in_store(
            self.rag_store.as_ref(),
            project_id,
            query,
            provider,
            top_k,
            filter,
        )
        .await
    }

    pub async fn hybrid_rag_search_with_embedding(
        &self,
        project_id: i64,
        query: &str,
        query_embedding: Vec<f32>,
        embedding_identity: &EmbeddingModelIdentity,
        top_k: usize,
        filter: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        hybrid_rag_search_with_embedding_in_store(
            self.rag_store.as_ref(),
            project_id,
            query,
            query_embedding,
            embedding_identity,
            top_k,
            filter,
        )
        .await
    }
}
