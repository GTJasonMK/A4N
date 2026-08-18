//! 对真实 Qdrant 的 VectorStore 合同烟雾（需 A4N_QDRANT_URL）。
//!
//! ```bash
//! ./scripts/dev/run-qdrant.sh
//! A4N_QDRANT_URL=http://127.0.0.1:6334 cargo test -p a4n-rag-index --test qdrant_live -- --nocapture
//! ```

use a4n_rag_core::vector_store::{DeleteFilter, EmbeddingRecord, SearchFilter, VectorStore};
use a4n_rag_index::{QdrantStoreConfig, QdrantVectorStore};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

fn require_qdrant() -> bool {
    std::env::var("A4N_QDRANT_URL")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

async fn store() -> Arc<QdrantVectorStore> {
    let coll = format!(
        "a4n_live_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let url = std::env::var("A4N_QDRANT_URL").expect("A4N_QDRANT_URL");
    QdrantVectorStore::connect(QdrantStoreConfig {
        url,
        api_key: std::env::var("A4N_QDRANT_API_KEY")
            .ok()
            .filter(|s| !s.is_empty()),
        collection: coll,
    })
    .await
    .expect("connect qdrant")
}

fn record(
    project_id: i64,
    id: &str,
    emb: Vec<f32>,
    content: &str,
    chapter: i64,
) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding: emb,
        metadata: json!({
            "project_id": project_id,
            "data_type": "chapter",
            "chapter_number": chapter,
        }),
        content: content.to_string(),
    }
}

#[tokio::test]
async fn live_ingest_query_text_hybrid_delete_completeness() {
    if !require_qdrant() {
        eprintln!("skip: set A4N_QDRANT_URL to run live Qdrant tests");
        return;
    }
    let store = store().await;
    let collection = "42";

    let invalid_collection = "invalid-content";
    let error = store
        .add_embeddings(
            invalid_collection,
            vec![record(42, "blank", vec![1.0, 0.0, 0.0], "   ", 1)],
        )
        .await
        .expect_err("blank content must reject");
    assert!(error.to_string().contains("空内容"));

    let expected_ids = vec!["not-indexed-yet".to_string()];
    assert!(
        store
            .metadata_by_ids(collection, &expected_ids)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .metadata_by_filter(
                collection,
                DeleteFilter {
                    ids: vec![],
                    id_prefixes: vec![],
                },
            )
            .await
            .unwrap()
            .is_empty()
    );
    let empty_report = store
        .completeness_report_by_ids(collection, &expected_ids)
        .await
        .unwrap();
    assert_eq!(empty_report.missing_ids, expected_ids);

    let n = store
        .add_embeddings(
            collection,
            vec![
                record(42, "doc-a", vec![1.0, 0.0, 0.0], "林晚施展血契之术", 1),
                record(42, "doc-b", vec![0.0, 1.0, 0.0], "普通章节叙述文本", 2),
                record(42, "doc-c", vec![0.0, 0.0, 1.0], "第三条无空格专名段落", 3),
            ],
        )
        .await
        .expect("ingest");
    assert_eq!(n, 3);

    let other_collection = "project/43";
    store
        .add_embeddings(
            other_collection,
            vec![record(
                43,
                "other-doc",
                vec![1.0, 0.0, 0.0, 0.0],
                "另一项目使用不同嵌入维度",
                1,
            )],
        )
        .await
        .expect("ingest second collection with different dimension");
    let vec_hits = store
        .query(collection, vec![1.0, 0.0, 0.0], 2, None)
        .await
        .expect("query");
    assert!(!vec_hits.is_empty());
    assert_eq!(vec_hits[0].id, "doc-a");

    let text_hits = store
        .search_text(collection, "血契", 5, None)
        .await
        .expect("search_text");
    assert!(
        text_hits.iter().any(|h| h.id == "doc-a"),
        "chinese proper noun should hit: {text_hits:?}"
    );

    let hybrid = store
        .hybrid_search(collection, "血契", vec![1.0, 0.0, 0.0], 5, None)
        .await
        .expect("hybrid");
    assert!(!hybrid.is_empty());

    let filtered = store
        .query(
            collection,
            vec![0.0, 1.0, 0.0],
            5,
            Some(SearchFilter {
                equals: Some(HashMap::from([("data_type".into(), json!("chapter"))])),
                chapter_range: Some((2, 2)),
                data_types: None,
            }),
        )
        .await
        .expect("filtered query");
    assert!(
        filtered
            .iter()
            .all(|h| h.id == "doc-b" || h.metadata["chapter_number"] == 2)
    );

    let report = store
        .completeness_report_by_ids(
            collection,
            &[
                "doc-a".into(),
                "doc-b".into(),
                "doc-c".into(),
                "missing".into(),
            ],
        )
        .await
        .expect("completeness");
    assert!(!report.missing_vector_ids.is_empty());
    assert!(report.missing_vector_ids.contains(&"missing".to_string()));
    assert!(!report.missing_ids.is_empty());
    assert_eq!(report.actual_vector, 3);
    assert_eq!(report.actual_bm25, 3);

    let deleted = store
        .delete(
            collection,
            DeleteFilter {
                ids: vec!["doc-b".to_string()],
                id_prefixes: vec![],
            },
        )
        .await
        .expect("delete exact record");
    assert_eq!(deleted, 1);

    store.drop_collection(collection).await.expect("drop");

    store
        .add_embeddings(
            collection,
            vec![record(
                42,
                "doc-rebuilt",
                vec![1.0, 0.0, 0.0, 0.0],
                "删除后以新维度重建",
                4,
            )],
        )
        .await
        .expect("rebuild collection with new dimension");
    let rebuilt = store
        .query(collection, vec![1.0, 0.0, 0.0, 0.0], 1, None)
        .await
        .expect("query rebuilt collection");
    assert_eq!(rebuilt[0].id, "doc-rebuilt");

    store
        .drop_collection(collection)
        .await
        .expect("drop rebuilt");
    store
        .drop_collection(other_collection)
        .await
        .expect("drop second collection");
}
