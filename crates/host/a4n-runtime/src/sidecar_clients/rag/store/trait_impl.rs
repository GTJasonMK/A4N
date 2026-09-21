use std::collections::HashMap;

use a4n_domain::ProviderError;
use a4n_rag_core::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use async_trait::async_trait;
use serde_json::Value;

use super::SidecarVectorStore;
use crate::provider_error::from_app_error;

#[async_trait]
impl VectorStore for SidecarVectorStore {
    async fn add_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, ProviderError> {
        self.ingest(collection, records)
            .await
            .map_err(from_app_error)
    }

    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, ProviderError> {
        SidecarVectorStore::query_typed(self, collection, query_embedding, top_k, filters)
            .await
            .map_err(from_app_error)
    }

    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, ProviderError> {
        SidecarVectorStore::search_text_typed(self, collection, query_text, top_k, filters)
            .await
            .map_err(from_app_error)
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, ProviderError> {
        SidecarVectorStore::hybrid_search_typed(
            self,
            collection,
            query_text,
            query_embedding,
            top_k,
            filters,
        )
        .await
        .map_err(from_app_error)
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, ProviderError> {
        SidecarVectorStore::delete_typed(self, collection, filter)
            .await
            .map_err(from_app_error)
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), ProviderError> {
        SidecarVectorStore::drop_collection_typed(self, collection)
            .await
            .map_err(from_app_error)
    }

    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, Value>, ProviderError> {
        SidecarVectorStore::metadata_by_ids_typed(self, collection, ids)
            .await
            .map_err(from_app_error)
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, Value>, ProviderError> {
        SidecarVectorStore::metadata_by_filter_typed(self, collection, filter)
            .await
            .map_err(from_app_error)
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, ProviderError> {
        SidecarVectorStore::completeness_report_by_ids_typed(self, collection, expected_ids)
            .await
            .map_err(from_app_error)
    }
}
