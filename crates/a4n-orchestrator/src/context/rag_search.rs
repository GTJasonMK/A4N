use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_rag_contract::RagSearchPort;
use a4n_rag_core::vector_store::SearchFilter;
use a4n_rag_core::{EnhancedRagQuery, EnhancedRagSearchResult, PreparedEnhancedRagSearch};
use async_trait::async_trait;

use super::OrchestratorContext;

#[async_trait]
impl RagSearchPort for OrchestratorContext {
    async fn embedding_model_identity(&self) -> Result<EmbeddingModelIdentity, AppError> {
        OrchestratorContext::embedding_model_identity(self).await
    }

    async fn prepare_enhanced_rag_queries(
        &self,
        project_id: i64,
        base_query: &str,
        chapter_number: Option<i32>,
    ) -> Result<Vec<EnhancedRagQuery>, AppError> {
        OrchestratorContext::prepare_enhanced_rag_queries(
            self,
            project_id,
            base_query,
            chapter_number,
        )
        .await
    }

    async fn enhanced_rag_search_filter(
        &self,
        project_id: i64,
        embedding_identity: &EmbeddingModelIdentity,
        data_types: Option<Vec<String>>,
    ) -> Result<SearchFilter, AppError> {
        OrchestratorContext::enhanced_rag_search_filter(
            self,
            project_id,
            embedding_identity,
            data_types,
        )
        .await
    }

    async fn execute_prepared_enhanced_rag_search(
        &self,
        prepared: PreparedEnhancedRagSearch,
    ) -> Result<EnhancedRagSearchResult, AppError> {
        OrchestratorContext::execute_prepared_enhanced_rag_search(self, prepared).await
    }

    async fn execute_prepared_enhanced_rag_search_with_vectors(
        &self,
        prepared: PreparedEnhancedRagSearch,
        embedding_identity: &EmbeddingModelIdentity,
        query_vectors: Vec<Vec<f32>>,
    ) -> Result<EnhancedRagSearchResult, AppError> {
        OrchestratorContext::execute_prepared_enhanced_rag_search_with_vectors(
            self,
            prepared,
            embedding_identity,
            query_vectors,
        )
        .await
    }
}
