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
        self.add_memory_embeddings(collection, records)
    }

    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.query_memory(collection, query_embedding, top_k, filters)
    }

    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.search_memory_text(collection, query_text, top_k, filters)
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.hybrid_memory_search(collection, query_text, query_embedding, top_k, filters)
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError> {
        Ok(self.delete_memory_records(collection, filter))
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        self.drop_memory_collection(collection);
        Ok(())
    }

    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        Ok(self.memory_metadata_by_ids(collection, ids))
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        Ok(self.memory_metadata_by_filter(collection, filter))
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        Ok(self.memory_completeness_report_by_ids(collection, expected_ids))
    }
}
