use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;
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
        AppError::VectorStoreError("unexpected ShortWriteVectorStore method call".to_string())
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
        _vector_weight: Option<f32>,
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

pub fn embedding_record(id: &str, data_type: &str, chapter_id: i64) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding: vec![1.0, 0.0],
        metadata: serde_json::json!({
            "project_id": 1,
            "data_type": data_type,
            "chapter_id": chapter_id,
            "chapter_number": 3,
        }),
        content: id.to_string(),
    }
}

pub fn root_text_metadata(project_id: i64) -> serde_json::Value {
    serde_json::json!({
        "root_project_id": project_id,
        "source_snapshot_id": null,
        "source_chapter_id": null,
        "source_snapshot_chapter_id": null,
        "derived_project_id": null,
    })
}

pub fn snapshot_text_metadata(
    root_project_id: i64,
    source_snapshot_id: i64,
    source_chapter_id: i64,
    source_snapshot_chapter_id: i64,
    data_type: &str,
) -> serde_json::Value {
    serde_json::json!({
        "root_project_id": root_project_id,
        "source_snapshot_id": source_snapshot_id,
        "source_chapter_id": source_chapter_id,
        "source_snapshot_chapter_id": source_snapshot_chapter_id,
        "derived_project_id": null,
        "data_type": data_type,
    })
}

#[cfg(test)]
mod tests;
