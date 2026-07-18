use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_rag_core::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use async_trait::async_trait;

use super::connection::LanceVectorStore;

#[async_trait]
impl VectorStore for LanceVectorStore {
    async fn add_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        self.add_embeddings_impl(collection, records).await
    }

    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.query_impl(collection, query_embedding, top_k, filters)
            .await
    }

    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.search_text_impl(collection, query_text, top_k, filters)
            .await
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
        vector_weight: Option<f32>,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.hybrid_search_impl(
            collection,
            query_text,
            query_embedding,
            top_k,
            filters,
            vector_weight,
        )
        .await
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError> {
        self.delete_impl(collection, filter).await
    }

    async fn collection_exists(&self, collection: &str) -> Result<bool, AppError> {
        self.collection_exists_impl(collection).await
    }

    async fn count(&self, collection: &str) -> Result<usize, AppError> {
        self.count_impl(collection).await
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        self.drop_collection_impl(collection).await
    }

    async fn list_collections(&self) -> Result<Vec<String>, AppError> {
        self.list_collections_impl().await
    }

    async fn update_metadata(
        &self,
        collection: &str,
        id: &str,
        metadata: serde_json::Value,
    ) -> Result<(), AppError> {
        self.update_metadata_impl(collection, id, metadata).await
    }

    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        self.metadata_by_ids_impl(collection, ids).await
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        self.metadata_by_filter_impl(collection, filter).await
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        self.health_check_impl().await
    }

    async fn completeness_report(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        self.completeness_report_impl(collection, expected_count)
            .await
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        self.completeness_report_by_ids_impl(collection, expected_ids)
            .await
    }
}
