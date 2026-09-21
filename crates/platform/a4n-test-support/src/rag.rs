use std::collections::HashMap;

use a4n_domain::ProviderError as AppError;
use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_rag_core::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};
use async_trait::async_trait;

pub struct ShortWriteVectorStore {
    written: usize,
}

impl ShortWriteVectorStore {
    pub fn new(written: usize) -> Self {
        Self { written }
    }

    fn unexpected_call() -> AppError {
        AppError::VectorStore("unexpected ShortWriteVectorStore method call".to_string())
    }
}

#[async_trait]
impl VectorStore for ShortWriteVectorStore {
    async fn add_embeddings(
        &self,
        _collection: &str,
        _records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        Ok(self.written)
    }

    async fn query(
        &self,
        _collection: &str,
        _query_embedding: Vec<f32>,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        Err(Self::unexpected_call())
    }

    async fn search_text(
        &self,
        _collection: &str,
        _query_text: &str,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        Err(Self::unexpected_call())
    }

    async fn hybrid_search(
        &self,
        _collection: &str,
        _query_text: &str,
        _query_embedding: Vec<f32>,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        Err(Self::unexpected_call())
    }

    async fn delete(&self, _collection: &str, _filter: DeleteFilter) -> Result<usize, AppError> {
        Err(Self::unexpected_call())
    }

    async fn drop_collection(&self, _collection: &str) -> Result<(), AppError> {
        Err(Self::unexpected_call())
    }

    async fn metadata_by_ids(
        &self,
        _collection: &str,
        _ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        Err(Self::unexpected_call())
    }

    async fn metadata_by_filter(
        &self,
        _collection: &str,
        _filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        Err(Self::unexpected_call())
    }

    async fn completeness_report_by_ids(
        &self,
        _collection: &str,
        _expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        Err(Self::unexpected_call())
    }
}

pub fn test_embedding_identity(model: &str) -> EmbeddingModelIdentity {
    EmbeddingModelIdentity {
        backend: "LocalHash".to_string(),
        model: model.to_string(),
        dimension: 2,
    }
}

pub fn embedding_record(id: &str) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding: vec![1.0, 0.0],
        metadata: serde_json::json!({}),
        content: id.to_string(),
    }
}

#[cfg(test)]
mod tests;
