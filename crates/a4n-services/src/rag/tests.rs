use super::{RagEmbeddingInput, RagService};
use a4n_core::error::AppError;
use a4n_core::rag::RAG_TEXT_INDEX_VERSION;
use a4n_engine::embedding::provider::{
    EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider,
};
use a4n_engine::rag::{
    memory_store::MemoryVectorStore,
    vector_store::{
        CompletenessReport, DeleteFilter, EmbeddingRecord, SearchFilter, SearchResult, VectorStore,
    },
};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

struct ShortEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for ShortEmbeddingProvider {
    async fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(vec![vec![1.0, 0.0]])
    }

    fn dimension(&self) -> usize {
        2
    }

    fn model_name(&self) -> &str {
        "short-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

struct ExtraEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for ExtraEmbeddingProvider {
    async fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(vec![vec![1.0, 0.0], vec![0.0, 1.0]])
    }

    fn dimension(&self) -> usize {
        2
    }

    fn model_name(&self) -> &str {
        "extra-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

struct MatchingEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for MatchingEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(texts
            .iter()
            .enumerate()
            .map(|(index, _)| vec![index as f32, 1.0])
            .collect())
    }

    fn dimension(&self) -> usize {
        2
    }

    fn model_name(&self) -> &str {
        "matching-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

struct WrongDimensionEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for WrongDimensionEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(texts.into_iter().map(|_| vec![1.0, 0.0]).collect())
    }

    fn dimension(&self) -> usize {
        3
    }

    fn model_name(&self) -> &str {
        "wrong-dimension-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

struct ShortWriteVectorStore {
    written: usize,
}

impl ShortWriteVectorStore {
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

    async fn collection_exists(&self, _collection: &str) -> Result<bool, AppError> {
        Err(Self::unexpected_call())
    }

    async fn create_collection(&self, _collection: &str) -> Result<(), AppError> {
        Err(Self::unexpected_call())
    }

    async fn count(&self, _collection: &str) -> Result<usize, AppError> {
        Err(Self::unexpected_call())
    }

    async fn drop_collection(&self, _collection: &str) -> Result<(), AppError> {
        Err(Self::unexpected_call())
    }

    async fn list_collections(&self) -> Result<Vec<String>, AppError> {
        Err(Self::unexpected_call())
    }

    async fn update_metadata(
        &self,
        _collection: &str,
        _id: &str,
        _metadata: serde_json::Value,
    ) -> Result<(), AppError> {
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

    async fn health_check(&self) -> Result<bool, AppError> {
        Err(Self::unexpected_call())
    }

    async fn completeness_report(
        &self,
        _collection: &str,
        _expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
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

fn test_embedding_identity(model: &str) -> EmbeddingModelIdentity {
    EmbeddingModelIdentity {
        backend: "LocalHash".to_string(),
        model: model.to_string(),
        dimension: 2,
    }
}

fn record(id: &str, data_type: &str, chapter_id: i64) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding: vec![1.0, 0.0],
        metadata: serde_json::json!({
            "project_id": 1,
            "data_type": data_type,
            "chapter_id": chapter_id,
            "chapter_number": 3,
        }),
        content: Some(id.to_string()),
    }
}

fn root_text_metadata(project_id: i64) -> serde_json::Value {
    serde_json::json!({
        "root_project_id": project_id,
        "source_snapshot_id": null,
        "source_chapter_id": null,
        "source_snapshot_chapter_id": null,
        "derived_project_id": null,
    })
}

fn snapshot_text_metadata(
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

#[test]
fn normalize_metadata_uses_argument_project_id() {
    let metadata = RagService::normalize_metadata(
        42,
        "正文内容",
        serde_json::json!({
            "root_project_id": 42,
            "source_snapshot_id": null,
            "source_chapter_id": null,
            "source_snapshot_chapter_id": null,
            "derived_project_id": null,
            "project_id": 7,
            "data_type": "chapter",
        }),
        &test_embedding_identity("matching-test"),
    )
    .expect("valid metadata");

    assert_eq!(metadata["project_id"], serde_json::json!(42));
    assert_eq!(metadata["root_project_id"], serde_json::json!(42));
    assert_eq!(metadata["source_snapshot_id"], serde_json::Value::Null);
    assert_eq!(metadata["derived_project_id"], serde_json::Value::Null);
    assert_eq!(metadata["data_type"], serde_json::json!("chapter"));
    assert_eq!(metadata["hash_algorithm"], serde_json::json!("sha256"));
    assert_eq!(
        metadata["text_index_version"],
        serde_json::json!(RAG_TEXT_INDEX_VERSION)
    );
    assert_eq!(
        metadata["embedding_backend"],
        serde_json::json!("LocalHash")
    );
    assert_eq!(
        metadata["embedding_model"],
        serde_json::json!("matching-test")
    );
    assert_eq!(metadata["embedding_dimension"], serde_json::json!(2));
    assert!(
        metadata["content_hash"]
            .as_str()
            .is_some_and(|value| value.len() == 64)
    );
}

#[test]
fn normalize_metadata_rejects_missing_project_family_context() {
    let err = RagService::normalize_metadata(
        42,
        "正文内容",
        serde_json::json!({
            "data_type": "chapter",
        }),
        &test_embedding_identity("matching-test"),
    )
    .expect_err("missing project family metadata should reject");

    assert!(err.to_string().contains("root_project_id"));
}

#[test]
fn normalize_metadata_rejects_cross_project_derived_context() {
    let err = RagService::normalize_metadata(
        8,
        "正文内容",
        serde_json::json!({
            "root_project_id": 1,
            "source_snapshot_id": 2,
            "source_chapter_id": 3,
            "source_snapshot_chapter_id": 4,
            "derived_project_id": 9,
        }),
        &test_embedding_identity("matching-test"),
    )
    .expect_err("derived_project_id must match collection project");

    assert!(
        err.to_string()
            .contains("derived_project_id 必须等于当前项目")
    );
}

#[test]
fn required_chunk_id_rejects_blank() {
    let err = RagService::required_chunk_id("  ").expect_err("blank id should reject");

    assert!(err.to_string().contains("chunk_id 不能为空"));
}

#[test]
fn required_content_rejects_blank() {
    let err = RagService::required_content("  ").expect_err("blank content should reject");

    assert!(err.to_string().contains("content 不能为空"));
}

#[test]
fn validate_embedding_count_rejects_mismatch() {
    let err = RagService::validate_embedding_count(&[vec![1.0, 0.0]], 2)
        .expect_err("mismatched embedding count should reject");

    assert!(err.to_string().contains("嵌入返回数量不匹配"));
}

#[tokio::test]
async fn embed_and_store_rejects_embedding_dimension_mismatch() {
    let store = Arc::new(MemoryVectorStore::new());
    let service = RagService::new(store.clone());
    let err = service
        .embed_and_store(
            1,
            "rag:wrong-dimension",
            "正文",
            &WrongDimensionEmbeddingProvider,
            root_text_metadata(1),
        )
        .await
        .expect_err("wrong embedding dimension should reject");

    assert!(err.to_string().contains("嵌入返回维度不匹配"));
    assert!(!store.collection_exists("1").await.unwrap());
}

#[tokio::test]
async fn embed_and_store_records_rejects_embedding_count_mismatch_without_writing_partial_records()
{
    let store = Arc::new(MemoryVectorStore::new());
    let service = RagService::new(store.clone());
    let err = service
        .embed_and_store_records(
            1,
            vec![
                RagEmbeddingInput {
                    chunk_id: "rag:1".to_string(),
                    text: "第一条".to_string(),
                    metadata: root_text_metadata(1),
                },
                RagEmbeddingInput {
                    chunk_id: "rag:2".to_string(),
                    text: "第二条".to_string(),
                    metadata: root_text_metadata(1),
                },
            ],
            &ShortEmbeddingProvider,
        )
        .await
        .expect_err("short provider response should reject");

    assert!(err.to_string().contains("嵌入返回数量不匹配"));
    assert!(!store.collection_exists("1").await.unwrap());
}

#[tokio::test]
async fn embed_and_store_records_rejects_vector_store_short_write() {
    let store = Arc::new(ShortWriteVectorStore { written: 1 });
    let service = RagService::new(store);
    let err = service
        .embed_and_store_records(
            1,
            vec![
                RagEmbeddingInput {
                    chunk_id: "rag:1".to_string(),
                    text: "第一条".to_string(),
                    metadata: root_text_metadata(1),
                },
                RagEmbeddingInput {
                    chunk_id: "rag:2".to_string(),
                    text: "第二条".to_string(),
                    metadata: root_text_metadata(1),
                },
            ],
            &MatchingEmbeddingProvider,
        )
        .await
        .expect_err("short vector-store write should reject");

    assert!(err.to_string().contains("向量存储写入数量不匹配"));
    assert!(err.to_string().contains("expected=2"));
    assert!(err.to_string().contains("actual=1"));
}

#[tokio::test]
async fn embed_and_store_records_preserves_per_record_metadata() {
    let store = Arc::new(MemoryVectorStore::new());
    let service = RagService::new(store.clone());

    let written = service
        .embed_and_store_records(
            9,
            vec![RagEmbeddingInput {
                chunk_id: "outline:9:1".to_string(),
                text: "正文内容".to_string(),
                metadata: serde_json::json!({
                    "root_project_id": 9,
                    "source_snapshot_id": null,
                    "source_chapter_id": null,
                    "source_snapshot_chapter_id": null,
                    "derived_project_id": null,
                    "data_type": "outline",
                    "chapter_number": 1,
                }),
            }],
            &MatchingEmbeddingProvider,
        )
        .await
        .expect("batch store");

    assert_eq!(written, 1);
    let records = store
        .query("9", vec![0.0, 1.0], 1, None)
        .await
        .expect("stored records should be queryable");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "正文内容");
    assert_eq!(records[0].metadata["project_id"], serde_json::json!(9));
    assert_eq!(records[0].metadata["root_project_id"], serde_json::json!(9));
    assert_eq!(
        records[0].metadata["source_snapshot_id"],
        serde_json::Value::Null
    );
    assert_eq!(
        records[0].metadata["derived_project_id"],
        serde_json::Value::Null
    );
    assert_eq!(
        records[0].metadata["data_type"],
        serde_json::json!("outline")
    );
    assert_eq!(records[0].metadata["chapter_number"], serde_json::json!(1));
    assert_eq!(
        records[0].metadata["hash_algorithm"],
        serde_json::json!("sha256")
    );
    assert_eq!(records[0].metadata["content_length"], serde_json::json!(4));
    assert_eq!(
        records[0].metadata["embedding_backend"],
        serde_json::json!("LocalHash")
    );
    assert_eq!(
        records[0].metadata["embedding_model"],
        serde_json::json!("matching-test")
    );
    assert_eq!(
        records[0].metadata["embedding_dimension"],
        serde_json::json!(2)
    );
    assert!(
        records[0].metadata["content_hash"]
            .as_str()
            .is_some_and(|value| value.len() == 64)
    );
}

#[tokio::test]
async fn hybrid_search_filtered_keeps_snapshot_scoped_results_isolated() {
    let store = Arc::new(MemoryVectorStore::new());
    let service = RagService::new(store);
    service
        .embed_and_store_records(
            1,
            vec![
                RagEmbeddingInput {
                    chunk_id: "snapshot:1:10:chapter:100".to_string(),
                    text: "同一个根项目的旧快照内容".to_string(),
                    metadata: snapshot_text_metadata(1, 10, 100, 1000, "chapter"),
                },
                RagEmbeddingInput {
                    chunk_id: "snapshot:1:20:chapter:100".to_string(),
                    text: "同一个根项目的新快照内容".to_string(),
                    metadata: snapshot_text_metadata(1, 20, 100, 2000, "chapter"),
                },
            ],
            &MatchingEmbeddingProvider,
        )
        .await
        .expect("snapshot records should store");

    let mut equals = HashMap::new();
    equals.insert("root_project_id".to_string(), serde_json::json!(1));
    equals.insert("source_snapshot_id".to_string(), serde_json::json!(20));
    let results = service
        .hybrid_search_filtered(
            1,
            "快照内容",
            &MatchingEmbeddingProvider,
            10,
            Some(SearchFilter {
                equals: Some(equals),
                chapter_range: None,
                data_types: Some(vec!["chapter".to_string()]),
            }),
        )
        .await
        .expect("search should succeed");

    let ids = results
        .into_iter()
        .map(|result| result.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["snapshot:1:20:chapter:100"]);
}

#[tokio::test]
async fn hybrid_search_filtered_rejects_query_embedding_count_mismatch() {
    let store = Arc::new(MemoryVectorStore::new());
    let service = RagService::new(store);
    let err = service
        .hybrid_search_filtered(1, "查询", &ExtraEmbeddingProvider, 10, None)
        .await
        .expect_err("extra query embeddings should reject");

    assert!(err.to_string().contains("嵌入返回数量不匹配"));
}

#[tokio::test]
async fn delete_chapter_by_id_removes_all_records_with_chapter_prefix() {
    let store = Arc::new(MemoryVectorStore::new());
    let service = RagService::new(store.clone());
    let collection = "1";
    store.create_collection(collection).await.unwrap();

    store
        .add_embeddings(
            collection,
            vec![
                record("chapter:1:100:p1", "chapter", 100),
                record("chapter:1:100:summary", "chapter_summary", 100),
                record("chapter:1:100:key_event:1", "key_event", 100),
                record("chapter:1:101:p1", "chapter", 101),
                record("outline:1:1", "outline", 0),
            ],
        )
        .await
        .unwrap();

    let deleted = service.delete_chapter_by_id(1, 100, 3).await.unwrap();
    assert_eq!(deleted, 3);

    let remaining = store
        .metadata_by_filter(
            collection,
            DeleteFilter {
                project_id: 1,
                data_type: None,
                chapter_number: None,
                ids: Vec::new(),
                id_prefixes: Vec::new(),
            },
        )
        .await
        .unwrap();
    let remaining_ids = remaining.keys().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(remaining_ids.len(), 2);
    assert!(remaining_ids.contains(&"chapter:1:101:p1"));
    assert!(remaining_ids.contains(&"outline:1:1"));
}
