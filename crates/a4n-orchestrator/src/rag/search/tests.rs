use std::{collections::HashMap, sync::Arc};

use a4n_rag_core::{
    memory_store::MemoryVectorStore,
    vector_store::{SearchFilter, VectorStore},
};
use a4n_test_support::{
    embedding::{ExtraEmbeddingProvider, MatchingEmbeddingProvider},
    rag::{embedding_record, snapshot_text_metadata, test_embedding_identity},
};

use super::execution::{
    hybrid_rag_search_filtered_in_store, hybrid_rag_search_with_embedding_in_store,
};
use super::filter::identity_scoped_search_filter;

#[test]
fn identity_filter_scopes_current_root_project_and_embedding() {
    let identity = test_embedding_identity("search-filter");
    let filter = identity_scoped_search_filter(
        7,
        identity.clone(),
        Some(vec!["chapter".to_string(), "key_event".to_string()]),
    );
    let equals = filter.equals.expect("identity filter equals");

    assert_eq!(equals["project_id"], serde_json::json!(7));
    assert_eq!(equals["root_project_id"], serde_json::json!(7));
    assert_eq!(equals["source_snapshot_id"], serde_json::Value::Null);
    assert_eq!(equals["derived_project_id"], serde_json::Value::Null);
    assert_eq!(equals["embedding_backend"], identity.backend);
    assert_eq!(equals["embedding_model"], identity.model);
    assert_eq!(equals["embedding_dimension"], identity.dimension);
    assert_eq!(
        filter.data_types,
        Some(vec!["chapter".to_string(), "key_event".to_string()])
    );
    assert_eq!(filter.chapter_range, None);
}

#[tokio::test]
async fn filtered_search_keeps_snapshot_scoped_results_isolated() {
    let store = Arc::new(MemoryVectorStore::new());
    store
        .add_embeddings(
            "1",
            vec![
                a4n_rag_core::vector_store::EmbeddingRecord {
                    id: "snapshot:1:10:chapter:100".to_string(),
                    embedding: vec![1.0, 0.0],
                    metadata: snapshot_text_metadata(1, 10, 100, 1000, "chapter"),
                    content: Some("同一个根项目的旧快照内容".to_string()),
                },
                a4n_rag_core::vector_store::EmbeddingRecord {
                    id: "snapshot:1:20:chapter:100".to_string(),
                    embedding: vec![1.0, 0.0],
                    metadata: snapshot_text_metadata(1, 20, 100, 2000, "chapter"),
                    content: Some("同一个根项目的新快照内容".to_string()),
                },
            ],
        )
        .await
        .expect("snapshot records should store");

    let mut equals = HashMap::new();
    equals.insert("root_project_id".to_string(), serde_json::json!(1));
    equals.insert("source_snapshot_id".to_string(), serde_json::json!(20));
    let results = hybrid_rag_search_filtered_in_store(
        store.as_ref(),
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
    .expect("filtered search should succeed");

    let ids = results
        .into_iter()
        .map(|result| result.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["snapshot:1:20:chapter:100"]);
}

#[tokio::test]
async fn provider_search_rejects_query_embedding_count_mismatch() {
    let store = MemoryVectorStore::new();
    let error =
        hybrid_rag_search_filtered_in_store(&store, 1, "查询", &ExtraEmbeddingProvider, 10, None)
            .await
            .expect_err("extra query embeddings should reject");

    assert!(error.to_string().contains("嵌入返回数量不匹配"));
}

#[tokio::test]
async fn caller_embedding_search_rejects_dimension_mismatch_before_store_access() {
    let store = MemoryVectorStore::new();
    let identity = test_embedding_identity("search-test");
    let error = hybrid_rag_search_with_embedding_in_store(
        &store,
        1,
        "查询",
        vec![1.0],
        &identity,
        10,
        None,
    )
    .await
    .expect_err("wrong query dimension should reject");

    assert!(error.to_string().contains("嵌入返回维度不匹配"));
}

#[tokio::test]
async fn caller_embedding_search_forwards_valid_vector_to_store() {
    let store = MemoryVectorStore::new();
    store
        .add_embeddings(
            "1",
            vec![embedding_record("chapter:1:100:p1", "chapter", 100)],
        )
        .await
        .expect("search record should store");

    let results = hybrid_rag_search_with_embedding_in_store(
        &store,
        1,
        "chapter",
        vec![1.0, 0.0],
        &test_embedding_identity("search-test"),
        10,
        None,
    )
    .await
    .expect("caller embedding search should succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "chapter:1:100:p1");
}
