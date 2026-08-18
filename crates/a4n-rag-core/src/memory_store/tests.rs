use super::MemoryVectorStore;
use crate::vector_store::{DeleteFilter, EmbeddingRecord, VectorStore};
use serde_json::json;

fn make_record(id: &str, content: &str, embedding: Vec<f32>) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding,
        content: content.to_string(),
        metadata: json!({"project_id": 1}),
    }
}

fn make_record_with_metadata(
    id: &str,
    content: &str,
    embedding: Vec<f32>,
    metadata: serde_json::Value,
) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding,
        content: content.to_string(),
        metadata,
    }
}

fn make_embedding(x: f32, y: f32, z: f32) -> Vec<f32> {
    vec![x, y, z]
}

#[tokio::test]
async fn test_add_and_query() {
    let store = MemoryVectorStore::new();

    store
        .add_embeddings(
            "test",
            vec![
                make_record("1", "苹果很好吃", make_embedding(1.0, 0.0, 0.0)),
                make_record("2", "香蕉也不错", make_embedding(0.0, 1.0, 0.0)),
                make_record("3", "电脑编程入门", make_embedding(0.0, 0.0, 1.0)),
            ],
        )
        .await
        .unwrap();

    let results = store
        .query("test", make_embedding(0.9, 0.1, 0.0), 2, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "1");
}

#[tokio::test]
async fn add_embeddings_rejects_mismatched_dimensions_in_collection() {
    let store = MemoryVectorStore::new();
    store
        .add_embeddings(
            "test",
            vec![make_record(
                "1",
                "苹果很好吃",
                make_embedding(1.0, 0.0, 0.0),
            )],
        )
        .await
        .unwrap();

    let err = store
        .add_embeddings("test", vec![make_record("2", "香蕉也不错", vec![0.0, 1.0])])
        .await
        .expect_err("mixed embedding dimensions should reject");

    assert!(err.to_string().contains("维度不一致"));
}

#[tokio::test]
async fn query_rejects_mismatched_query_dimension() {
    let store = MemoryVectorStore::new();
    store
        .add_embeddings(
            "test",
            vec![make_record(
                "1",
                "苹果很好吃",
                make_embedding(1.0, 0.0, 0.0),
            )],
        )
        .await
        .unwrap();

    let err = store
        .query("test", vec![1.0, 0.0], 1, None)
        .await
        .expect_err("query dimension mismatch should reject");

    assert!(err.to_string().contains("query_dimension=2"));
    assert!(err.to_string().contains("record_dimension=3"));
}

#[tokio::test]
async fn test_text_search() {
    let store = MemoryVectorStore::new();

    store
        .add_embeddings(
            "test",
            vec![
                make_record("1", "苹果很好吃", make_embedding(1.0, 0.0, 0.0)),
                make_record("2", "香蕉也不错", make_embedding(0.0, 1.0, 0.0)),
                make_record("3", "苹果电脑编程入门", make_embedding(0.0, 0.0, 1.0)),
            ],
        )
        .await
        .unwrap();

    let results = store.search_text("test", "苹果", 5, None).await.unwrap();
    assert!(!results.is_empty());
    let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"1"));
    assert!(ids.contains(&"3"));
}

#[tokio::test]
async fn test_hybrid_search() {
    let store = MemoryVectorStore::new();

    store
        .add_embeddings(
            "test",
            vec![
                make_record("1", "苹果很好吃", make_embedding(1.0, 0.0, 0.0)),
                make_record("2", "香蕉也不错", make_embedding(0.0, 1.0, 0.0)),
                make_record("3", "苹果电脑编程入门", make_embedding(0.0, 0.0, 1.0)),
            ],
        )
        .await
        .unwrap();

    let results = store
        .hybrid_search("test", "苹果", make_embedding(0.9, 0.1, 0.0), 3, None)
        .await
        .unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_delete_with_filter() {
    let store = MemoryVectorStore::new();

    store
        .add_embeddings(
            "test",
            vec![
                make_record("1", "doc one", make_embedding(1.0, 0.0, 0.0)),
                make_record("2", "doc two", make_embedding(0.0, 1.0, 0.0)),
            ],
        )
        .await
        .unwrap();

    let deleted = store
        .delete(
            "test",
            DeleteFilter {
                ids: Vec::new(),
                id_prefixes: Vec::new(),
            },
        )
        .await
        .unwrap();
    assert_eq!(deleted, 2);
    assert!(
        store
            .query("test", make_embedding(1.0, 0.0, 0.0), 1, None)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn test_delete_with_id_prefix_removes_metadata_mismatch_record() {
    let store = MemoryVectorStore::new();

    store
        .add_embeddings(
            "test",
            vec![
                make_record_with_metadata(
                    "chapter:1:100:p1",
                    "chapter text with mismatched metadata",
                    make_embedding(1.0, 0.0, 0.0),
                    json!({
                        "project_id": 0,
                        "data_type": "chapter",
                        "chapter_number": 8,
                    }),
                ),
                make_record_with_metadata(
                    "chapter:1:101:p1",
                    "other chapter text",
                    make_embedding(0.0, 1.0, 0.0),
                    json!({
                        "project_id": 1,
                        "data_type": "chapter",
                        "chapter_number": 4,
                    }),
                ),
                make_record_with_metadata(
                    "chapter:1:102:p1",
                    "metadata matches but id prefix does not",
                    make_embedding(0.0, 0.0, 1.0),
                    json!({
                        "project_id": 1,
                        "data_type": "chapter",
                        "chapter_number": 3,
                    }),
                ),
            ],
        )
        .await
        .unwrap();

    let deleted = store
        .delete(
            "test",
            DeleteFilter {
                ids: Vec::new(),
                id_prefixes: vec!["chapter:1:100:".to_string()],
            },
        )
        .await
        .unwrap();

    assert_eq!(deleted, 1);
    let remaining = store
        .query("test", make_embedding(0.0, 1.0, 0.0), 10, None)
        .await
        .unwrap();
    let remaining_ids = remaining
        .iter()
        .map(|record| record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(remaining_ids.len(), 2);
    assert!(remaining_ids.contains(&"chapter:1:101:p1"));
    assert!(remaining_ids.contains(&"chapter:1:102:p1"));
}

#[tokio::test]
async fn completeness_report_by_ids_lists_concrete_missing_records() {
    let store = MemoryVectorStore::new();
    store
        .add_embeddings(
            "test",
            vec![
                make_record(
                    "chapter:1:1:p1",
                    "第一段正文",
                    make_embedding(1.0, 0.0, 0.0),
                ),
                make_record(
                    "chapter:1:1:summary",
                    "章节摘要",
                    make_embedding(0.0, 1.0, 0.0),
                ),
            ],
        )
        .await
        .unwrap();

    let report = store
        .completeness_report_by_ids(
            "test",
            &[
                "chapter:1:1:p1".to_string(),
                "chapter:1:1:p2".to_string(),
                "chapter:1:1:summary".to_string(),
            ],
        )
        .await
        .unwrap();

    assert!(!report.is_complete);
    assert_eq!(report.expected, 3);
    assert_eq!(report.actual_vector, 2);
    assert_eq!(report.actual_bm25, 2);
    assert_eq!(report.missing_vector_ids, vec!["chapter:1:1:p2"]);
    assert_eq!(report.missing_bm25_ids, vec!["chapter:1:1:p2"]);
    assert_eq!(report.missing_ids, vec!["chapter:1:1:p2"]);
}

#[tokio::test]
async fn completeness_report_by_ids_marks_all_expected_missing_for_absent_collection() {
    let store = MemoryVectorStore::new();

    let report = store
        .completeness_report_by_ids(
            "missing",
            &["outline:1:1".to_string(), "blueprint:1:1".to_string()],
        )
        .await
        .unwrap();

    assert!(!report.is_complete);
    assert_eq!(report.actual_vector, 0);
    assert_eq!(report.actual_bm25, 0);
    assert_eq!(
        report.missing_ids,
        vec!["blueprint:1:1".to_string(), "outline:1:1".to_string()]
    );
}

#[tokio::test]
async fn collection_lifecycle_follows_persisted_records() {
    let store = MemoryVectorStore::new();
    store
        .add_embeddings(
            "new_coll",
            vec![make_record(
                "record-1",
                "collection lifecycle",
                make_embedding(1.0, 0.0, 0.0),
            )],
        )
        .await
        .unwrap();
    store.drop_collection("new_coll").await.unwrap();
    let error = store
        .query("new_coll", make_embedding(1.0, 0.0, 0.0), 1, None)
        .await
        .expect_err("dropped collection must not remain queryable");
    assert!(error.to_string().contains("集合不存在"));
}
