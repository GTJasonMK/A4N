use super::*;
use serde_json::json;
use std::collections::HashMap;

fn rec(id: &str, embedding: Vec<f32>, project_id: i64, content: &str) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding,
        metadata: serde_json::json!({
            "project_id": project_id,
            "data_type": "chapter",
            "chapter_number": 1,
        }),
        content: Some(content.to_string()),
    }
}

fn rec_with_metadata(
    id: &str,
    embedding: Vec<f32>,
    content: &str,
    metadata: serde_json::Value,
) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding,
        metadata,
        content: Some(content.to_string()),
    }
}

fn search_filter_equals(key: &str, value: serde_json::Value) -> SearchFilter {
    SearchFilter {
        equals: Some(HashMap::from([(key.to_string(), value)])),
        chapter_range: None,
        data_types: None,
    }
}

#[tokio::test]
async fn add_and_query_basic() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();

    let records = vec![
        rec("a", vec![1.0, 0.0, 0.0, 0.0], 1, "alpha"),
        rec("b", vec![0.0, 1.0, 0.0, 0.0], 1, "beta"),
        rec("c", vec![0.0, 0.0, 1.0, 0.0], 1, "gamma"),
    ];
    let n = store.add_embeddings("proj1", records).await.unwrap();
    assert_eq!(n, 3);

    let r = store
        .query("proj1", vec![1.0, 0.0, 0.0, 0.0], 2, None)
        .await
        .unwrap();
    assert!(!r.is_empty());
    assert_eq!(r[0].id, "a", "查询向量应当最贴近 a");
}

#[tokio::test]
async fn add_rejects_mismatched_dimensions_in_batch() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();

    let err = store
        .add_embeddings(
            "bad_batch",
            vec![
                rec("a", vec![1.0, 0.0, 0.0], 1, "alpha"),
                rec("b", vec![0.0, 1.0], 1, "beta"),
            ],
        )
        .await
        .expect_err("mixed vector dimensions should reject");

    assert!(err.to_string().contains("不同维度"));
}

#[tokio::test]
async fn query_rejects_mismatched_dimension() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();
    store
        .add_embeddings("dim_query", vec![rec("a", vec![1.0, 0.0, 0.0], 1, "alpha")])
        .await
        .unwrap();

    let err = store
        .query("dim_query", vec![1.0, 0.0], 1, None)
        .await
        .expect_err("query vector dimension mismatch should reject");

    assert!(err.to_string().contains("query_dimension=2"));
    assert!(err.to_string().contains("collection_dimension=3"));
}

#[tokio::test]
async fn count_and_drop() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();

    let records = vec![
        rec("x", vec![1.0, 0.0], 9, "x"),
        rec("y", vec![0.0, 1.0], 9, "y"),
    ];
    store.add_embeddings("c1", records).await.unwrap();
    assert_eq!(store.count("c1").await.unwrap(), 2);
    assert!(store.collection_exists("c1").await.unwrap());

    store.drop_collection("c1").await.unwrap();
    assert!(!store.collection_exists("c1").await.unwrap());
    assert_eq!(store.count("c1").await.unwrap(), 0);
}

#[tokio::test]
async fn delete_by_project_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();

    let records = vec![
        rec("p1-a", vec![1.0, 0.0], 1, "p1a"),
        rec("p2-b", vec![0.0, 1.0], 2, "p2b"),
    ];
    store.add_embeddings("mixed", records).await.unwrap();
    let removed = store
        .delete(
            "mixed",
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
    assert_eq!(removed, 1);
    assert_eq!(store.count("mixed").await.unwrap(), 1);
}

#[tokio::test]
async fn delete_by_id_prefix_removes_metadata_mismatch_record() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();

    let records = vec![
        rec_with_metadata(
            "chapter:1:100:p1",
            vec![1.0, 0.0],
            "chapter text with mismatched metadata",
            serde_json::json!({
                "project_id": 0,
                "data_type": "chapter",
                "chapter_number": 8,
            }),
        ),
        rec_with_metadata(
            "chapter:1:101:p1",
            vec![0.0, 1.0],
            "other chapter text",
            serde_json::json!({
                "project_id": 1,
                "data_type": "chapter",
                "chapter_number": 4,
            }),
        ),
    ];
    store
        .add_embeddings("prefix_delete", records)
        .await
        .unwrap();

    let removed = store
        .delete(
            "prefix_delete",
            DeleteFilter {
                project_id: 1,
                data_type: Some("chapter".to_string()),
                chapter_number: Some(3),
                ids: Vec::new(),
                id_prefixes: vec!["chapter:1:100:".to_string()],
            },
        )
        .await
        .unwrap();

    assert_eq!(removed, 1);
    assert_eq!(store.count("prefix_delete").await.unwrap(), 1);
}

#[tokio::test]
async fn query_applies_generic_metadata_equals_before_top_k_truncation() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();

    store
        .add_embeddings(
            "metadata_filter",
            vec![
                rec_with_metadata(
                    "near-wrong-character",
                    vec![1.0, 0.0],
                    "near vector but wrong character",
                    json!({
                        "project_id": 1,
                        "data_type": "character_note",
                        "character_name": "Bob",
                    }),
                ),
                rec_with_metadata(
                    "far-matching-character",
                    vec![0.0, 1.0],
                    "far vector but matching character",
                    json!({
                        "project_id": 1,
                        "data_type": "character_note",
                        "character_name": "Alice",
                    }),
                ),
            ],
        )
        .await
        .unwrap();

    let results = store
        .query(
            "metadata_filter",
            vec![1.0, 0.0],
            1,
            Some(search_filter_equals("character_name", json!("Alice"))),
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "far-matching-character");
}

#[tokio::test]
async fn query_with_empty_data_types_matches_no_records() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();

    store
        .add_embeddings(
            "empty_types",
            vec![rec_with_metadata(
                "chapter:1:1:p1",
                vec![1.0, 0.0],
                "chapter text",
                json!({
                    "project_id": 1,
                    "data_type": "chapter",
                }),
            )],
        )
        .await
        .unwrap();

    let results = store
        .query(
            "empty_types",
            vec![1.0, 0.0],
            1,
            Some(SearchFilter {
                equals: None,
                chapter_range: None,
                data_types: Some(Vec::new()),
            }),
        )
        .await
        .unwrap();

    assert!(results.is_empty());
}

#[tokio::test]
async fn upsert_overwrites_existing_id() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();

    store
        .add_embeddings("up", vec![rec("same-id", vec![1.0, 0.0], 1, "v1")])
        .await
        .unwrap();
    store
        .add_embeddings("up", vec![rec("same-id", vec![0.0, 1.0], 1, "v2")])
        .await
        .unwrap();
    assert_eq!(store.count("up").await.unwrap(), 1, "upsert 不应增加行数");

    let r = store.query("up", vec![0.0, 1.0], 1, None).await.unwrap();
    assert_eq!(r[0].content, "v2", "查询到的应当是覆盖后的版本");
}

#[tokio::test]
async fn list_collections_works() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LanceVectorStore::open(tmp.path().to_path_buf())
        .await
        .unwrap();
    store
        .add_embeddings("a", vec![rec("1", vec![1.0, 0.0], 1, "a")])
        .await
        .unwrap();
    store
        .add_embeddings("b", vec![rec("1", vec![1.0, 0.0], 1, "b")])
        .await
        .unwrap();
    let mut names = store.list_collections().await.unwrap();
    names.sort();
    assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
}
