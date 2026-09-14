use std::collections::HashMap;

use a4n_domain::ProviderError;
use a4n_rag_core::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use async_trait::async_trait;
use serde_json::{Value, json};

use super::SidecarVectorStore;
use super::transport::RagOperation;
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
        self.call(
            RagOperation::Query,
            json!({
                "collection": collection,
                "query_embedding": query_embedding,
                "top_k": top_k,
                "filters": filters
            }),
        )
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
        self.call(
            RagOperation::SearchText,
            json!({
                "collection": collection,
                "query_text": query_text,
                "top_k": top_k,
                "filters": filters
            }),
        )
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
        self.call(
            RagOperation::Search,
            json!({
                "collection": collection,
                "query_text": query_text,
                "query_embedding": query_embedding,
                "top_k": top_k,
                "filters": filters
            }),
        )
        .await
        .map_err(from_app_error)
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, ProviderError> {
        self.call(
            RagOperation::Delete,
            json!({ "collection": collection, "filter": filter }),
        )
        .await
        .map_err(from_app_error)
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), ProviderError> {
        self.call(
            RagOperation::DropCollection,
            json!({ "collection": collection }),
        )
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
        self.call(
            RagOperation::MetadataByFilter,
            json!({ "collection": collection, "filter": filter }),
        )
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
