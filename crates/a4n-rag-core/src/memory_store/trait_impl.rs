use std::collections::HashMap;

use async_trait::async_trait;

use a4n_core::error::AppError;

use crate::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};

use super::MemoryVectorStore;

#[async_trait]
impl VectorStore for MemoryVectorStore {
    async fn add_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        self.add_memory_embeddings(collection, records).await
    }

    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.query_memory(collection, query_embedding, top_k, filters)
            .await
    }

    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.search_memory_text(collection, query_text, top_k, filters)
            .await
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
        _vector_weight: Option<f32>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.hybrid_memory_search(collection, query_text, query_embedding, top_k, filters)
            .await
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError> {
        self.delete_memory_records(collection, filter).await
    }

    async fn collection_exists(&self, collection: &str) -> Result<bool, AppError> {
        Ok(self.memory_collection_exists(collection).await)
    }

    async fn count(&self, collection: &str) -> Result<usize, AppError> {
        self.memory_collection_count(collection).await
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        self.drop_memory_collection(collection).await;
        Ok(())
    }

    async fn list_collections(&self) -> Result<Vec<String>, AppError> {
        Ok(self.list_memory_collections().await)
    }

    async fn update_metadata(
        &self,
        collection: &str,
        id: &str,
        metadata: serde_json::Value,
    ) -> Result<(), AppError> {
        self.update_memory_metadata(collection, id, metadata).await
    }

    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        Ok(self.memory_metadata_by_ids(collection, ids).await)
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        Ok(self.memory_metadata_by_filter(collection, filter).await)
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        Ok(true)
    }

    async fn completeness_report(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        self.memory_completeness_report(collection, expected_count)
            .await
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        Ok(self
            .memory_completeness_report_by_ids(collection, expected_ids)
            .await)
    }
}
