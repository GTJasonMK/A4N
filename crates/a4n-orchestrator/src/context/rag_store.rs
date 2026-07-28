use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingModelIdentity, EmbeddingProvider};
use a4n_rag_core::vector_store::{DeleteFilter, SearchFilter, SearchResult};
use serde_json::Value;

use super::OrchestratorContext;
use crate::rag::{
    RagEmbeddedInput, RagEmbeddingInput, embed_and_store_rag_record_in_store,
    embed_and_store_rag_records_in_store, hybrid_rag_search_filtered_in_store,
    hybrid_rag_search_with_embedding_in_store, store_embedded_rag_records_in_store,
};

impl OrchestratorContext {
    pub(crate) async fn rag_search_filtered_with_provider(
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

    pub(crate) async fn rag_search_with_embedding_vector(
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

    pub(crate) async fn rag_embed_and_store_record(
        &self,
        project_id: i64,
        chunk_id: &str,
        text: &str,
        metadata: Value,
    ) -> Result<Vec<f32>, AppError> {
        embed_and_store_rag_record_in_store(
            self.rag_store.as_ref(),
            project_id,
            chunk_id,
            text,
            self.embedding.as_ref(),
            metadata,
        )
        .await
    }

    pub(crate) async fn rag_embed_and_store_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddingInput>,
    ) -> Result<usize, AppError> {
        embed_and_store_rag_records_in_store(
            self.rag_store.as_ref(),
            project_id,
            inputs,
            self.embedding.as_ref(),
        )
        .await
    }

    pub(crate) async fn rag_store_embedded_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddedInput>,
        embedding_identity: EmbeddingModelIdentity,
    ) -> Result<usize, AppError> {
        store_embedded_rag_records_in_store(
            self.rag_store.as_ref(),
            project_id,
            inputs,
            embedding_identity,
        )
        .await
    }

    pub(crate) async fn rag_drop_project_collection(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        self.rag_store
            .drop_collection(&project_id.to_string())
            .await
    }

    pub(crate) async fn rag_delete_project_records(
        &self,
        project_id: i64,
        filter: DeleteFilter,
    ) -> Result<usize, AppError> {
        self.rag_store.delete(&project_id.to_string(), filter).await
    }

    pub(crate) async fn rag_project_metadata_by_filter(
        &self,
        project_id: i64,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, Value>, AppError> {
        self.rag_store
            .metadata_by_filter(&project_id.to_string(), filter)
            .await
    }

    pub(crate) async fn rag_project_metadata_by_ids(
        &self,
        project_id: i64,
        ids: &[String],
    ) -> Result<HashMap<String, Value>, AppError> {
        self.rag_store
            .metadata_by_ids(&project_id.to_string(), ids)
            .await
    }

    pub(crate) async fn execute_prepared_chapter_generation_rag_search(
        &self,
        prepared: &crate::chapter_generation::PreparedGenerationRagSearch,
    ) -> Result<crate::chapter_generation::GenerationRagContext, String> {
        crate::chapter_generation::rag_context::execute_prepared_chapter_generation_rag_search_in_store(
            self.rag_store.as_ref(),
            self.embedding.as_ref(),
            prepared,
        )
        .await
    }

    pub(crate) async fn execute_prepared_chapter_generation_rag_search_with_vectors(
        &self,
        prepared: &crate::chapter_generation::PreparedGenerationRagSearch,
        embedding_identity: &a4n_embedding::provider::EmbeddingModelIdentity,
        query_vectors: Vec<Vec<f32>>,
    ) -> Result<crate::chapter_generation::GenerationRagContext, String> {
        crate::chapter_generation::rag_context::execute_prepared_chapter_generation_rag_search_with_vectors_in_store(
            self.rag_store.as_ref(),
            prepared,
            embedding_identity,
            query_vectors,
        )
        .await
    }

    pub async fn execute_prepared_enhanced_rag_search(
        &self,
        project_id: i64,
        prepared: &crate::rag::PreparedEnhancedRagSearch,
    ) -> Result<crate::rag::EnhancedRagSearchResult, a4n_core::error::AppError> {
        crate::rag::search::enhanced::execute_prepared_enhanced_rag_search_in_store(
            self.rag_store.as_ref(),
            self.embedding.as_ref(),
            project_id,
            prepared,
        )
        .await
    }

    pub async fn execute_prepared_enhanced_rag_search_with_vectors(
        &self,
        project_id: i64,
        prepared: &crate::rag::PreparedEnhancedRagSearch,
        embedding_identity: &a4n_embedding::provider::EmbeddingModelIdentity,
        query_vectors: Vec<Vec<f32>>,
    ) -> Result<crate::rag::EnhancedRagSearchResult, a4n_core::error::AppError> {
        crate::rag::search::enhanced::execute_prepared_enhanced_rag_search_with_vectors_in_store(
            self.rag_store.as_ref(),
            project_id,
            prepared,
            embedding_identity,
            query_vectors,
        )
        .await
    }
}
