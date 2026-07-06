use std::collections::HashMap;
use std::sync::Arc;

use a4n_core::error::AppError;
use async_trait::async_trait;
use serde_json::json;

use super::HybridVectorStore;
use crate::rag::memory_store::MemoryVectorStore;
use crate::rag::tantivy_index::{TantivyTextIndex, TantivyTextRecord};
use crate::rag::vector_store::{
    CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
};

fn record(id: &str, content: &str, data_type: &str, embedding: Vec<f32>) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding,
        metadata: json!({
            "project_id": 1,
            "data_type": data_type,
            "chapter_number": 1,
        }),
        content: Some(content.to_string()),
    }
}

struct FailingVectorStore;

fn vector_error(label: &str) -> AppError {
    AppError::VectorStoreError(label.to_string())
}

#[async_trait]
impl VectorStore for FailingVectorStore {
    async fn add_embeddings(
        &self,
        _collection: &str,
        _records: Vec<EmbeddingRecord>,
    ) -> Result<usize, AppError> {
        Err(vector_error("add failed"))
    }

    async fn query(
        &self,
        _collection: &str,
        _query_embedding: Vec<f32>,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        Err(vector_error("query failed"))
    }

    async fn search_text(
        &self,
        _collection: &str,
        _query_text: &str,
        _top_k: usize,
        _filters: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        Err(vector_error("search failed"))
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
        Err(vector_error("hybrid failed"))
    }

    async fn delete(&self, _collection: &str, _filter: DeleteFilter) -> Result<usize, AppError> {
        Err(vector_error("delete failed"))
    }

    async fn collection_exists(&self, _collection: &str) -> Result<bool, AppError> {
        Err(vector_error("exists failed"))
    }

    async fn create_collection(&self, _collection: &str) -> Result<(), AppError> {
        Err(vector_error("create failed"))
    }

    async fn count(&self, _collection: &str) -> Result<usize, AppError> {
        Err(vector_error("count failed"))
    }

    async fn drop_collection(&self, _collection: &str) -> Result<(), AppError> {
        Err(vector_error("drop failed"))
    }

    async fn list_collections(&self) -> Result<Vec<String>, AppError> {
        Err(vector_error("list failed"))
    }

    async fn update_metadata(
        &self,
        _collection: &str,
        _id: &str,
        _metadata: serde_json::Value,
    ) -> Result<(), AppError> {
        Err(vector_error("update failed"))
    }

    async fn metadata_by_ids(
        &self,
        _collection: &str,
        _ids: &[String],
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        Err(vector_error("metadata by ids failed"))
    }

    async fn metadata_by_filter(
        &self,
        _collection: &str,
        _filter: DeleteFilter,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        Err(vector_error("metadata by filter failed"))
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        Err(vector_error("health failed"))
    }

    async fn completeness_report(
        &self,
        _collection: &str,
        _expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        Err(vector_error("completeness failed"))
    }

    async fn completeness_report_by_ids(
        &self,
        _collection: &str,
        _expected_ids: &[String],
    ) -> Result<CompletenessReport, AppError> {
        Err(vector_error("completeness by ids failed"))
    }
}

#[tokio::test]
async fn hybrid_search_propagates_vector_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let text = Arc::new(TantivyTextIndex::new(tmp.path().to_path_buf()).unwrap());
    text.upsert(
        "proj",
        vec![TantivyTextRecord::new("doc-1", "needle text", Some(1))],
    )
    .await
    .unwrap();
    let store = HybridVectorStore::new(Arc::new(FailingVectorStore), text);

    let err = store
        .hybrid_search("proj", "needle", vec![1.0, 0.0], 10, None, None)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("query failed"));
}

#[tokio::test]
async fn hybrid_search_keeps_bm25_matches_that_pass_metadata_filtering() {
    let tmp = tempfile::tempdir().unwrap();
    let text = Arc::new(TantivyTextIndex::new(tmp.path().to_path_buf()).unwrap());
    let inner = Arc::new(MemoryVectorStore::new());
    let store = HybridVectorStore::new(inner, text);
    store.create_collection("1").await.unwrap();
    store
        .add_embeddings(
            "1",
            vec![
                record(
                    "chapter:1:1:p1",
                    "needle allowed chapter",
                    "chapter",
                    vec![1.0, 0.0],
                ),
                record(
                    "outline:1:1",
                    "needle forbidden outline",
                    "outline",
                    vec![0.0, 1.0],
                ),
            ],
        )
        .await
        .unwrap();

    let results = store
        .hybrid_search(
            "1",
            "needle",
            vec![0.0, 1.0],
            10,
            Some(SearchFilter {
                equals: Some(HashMap::from([("project_id".to_string(), json!(1))])),
                chapter_range: None,
                data_types: Some(vec!["chapter".to_string()]),
            }),
            None,
        )
        .await
        .unwrap();

    let ids = results
        .iter()
        .map(|result| result.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["chapter:1:1:p1"]);
}
