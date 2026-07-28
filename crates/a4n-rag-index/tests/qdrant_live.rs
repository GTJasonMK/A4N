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

fn record(id: &str, emb: Vec<f32>, content: &str, chapter: i64) -> EmbeddingRecord {
    EmbeddingRecord {
        id: id.to_string(),
        embedding: emb,
        metadata: json!({
            "project_id": 42,
            "data_type": "chapter",
            "chapter_number": chapter,
        }),
        content: Some(content.to_string()),
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

    let n = store
        .add_embeddings(
            collection,
            vec![
                record("doc-a", vec![1.0, 0.0, 0.0], "林晚施展血契之术", 1),
                record("doc-b", vec![0.0, 1.0, 0.0], "普通章节叙述文本", 2),
                record("doc-c", vec![0.0, 0.0, 1.0], "第三条无空格专名段落", 3),
            ],
        )
        .await
        .expect("ingest");
    assert_eq!(n, 3);
    assert!(store.collection_exists(collection).await.unwrap());
    assert_eq!(store.count(collection).await.unwrap(), 3);

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
        .hybrid_search(collection, "血契", vec![1.0, 0.0, 0.0], 5, None, Some(0.5))
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

    let deleted = store
        .delete(
            collection,
            DeleteFilter {
                project_id: 42,
                data_type: None,
                chapter_number: Some(2),
                ids: vec![],
                id_prefixes: vec![],
            },
        )
        .await
        .expect("delete chapter 2");
    assert!(deleted >= 1);
    assert_eq!(store.count(collection).await.unwrap(), 2);

    let by_prefix = store
        .delete(
            collection,
            DeleteFilter {
                project_id: 42,
                data_type: None,
                chapter_number: None,
                ids: vec![],
                id_prefixes: vec!["doc-".into()],
            },
        )
        .await
        .expect("delete by prefix");
    // id_prefix payload stores prefixes of chunk ids; "doc-" may match
    let _ = by_prefix;

    store.drop_collection(collection).await.expect("drop");
    assert!(!store.collection_exists(collection).await.unwrap());
    assert_eq!(store.count(collection).await.unwrap(), 0);

    assert!(store.health_check().await.unwrap());
}
