use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_rag_core::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use async_trait::async_trait;
use serde_json::{Value, json};

use super::SidecarVectorStore;
use super::transport::RagOperation;

#[async_trait]
impl VectorStore for SidecarVectorStore {
    async fn add_embeddings(
        &self,
        collection: &str,
        records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        self.ingest(collection, records).await
    }

    async fn query(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
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
    }

    async fn search_text(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
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
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
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
    }

    async fn delete(&self, collection: &str, filter: DeleteFilter) -> Result<usize, AppError> {
        self.call(
            RagOperation::Delete,
            json!({ "collection": collection, "filter": filter }),
        )
        .await
    }

    async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        self.call(
            RagOperation::DropCollection,
            json!({ "collection": collection }),
        )
        .await
    }

    async fn metadata_by_ids(
        &self,
        collection: &str,
        ids: &[String],
    ) -> Result<HashMap<String, Value>, AppError> {
        self.call(
            RagOperation::MetadataByIds,
            json!({ "collection": collection, "ids": ids }),
        )
        .await
    }

    async fn metadata_by_filter(
        &self,
        collection: &str,
        filter: DeleteFilter,
    ) -> Result<HashMap<String, Value>, AppError> {
        self.call(
            RagOperation::MetadataByFilter,
            json!({ "collection": collection, "filter": filter }),
        )
        .await
    }

    async fn completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        self.call(
            RagOperation::CompletenessReportByIds,
            json!({ "collection": collection, "expected_ids": expected_ids }),
        )
        .await
    }
}
