use a4n_rag_core::{
    memory_store::MemoryVectorStore,
    vector_store::VectorStore,
};
use a4n_test_support::{
    embedding::ExtraEmbeddingProvider,
    rag::{embedding_record, test_embedding_identity},
};

use super::execution::{
    hybrid_rag_search_filtered_in_store,
    hybrid_rag_search_with_validated_embedding_in_store,
};
use super::filter::identity_scoped_search_filter;

#[test]
fn identity_filter_scopes_embedding_and_data_types() {
    let identity = test_embedding_identity("search-filter");
    let filter = identity_scoped_search_filter(
        identity.clone(),
        Some(vec!["chapter".to_string(), "key_event".to_string()]),
    );
    let equals = filter.equals.expect("identity filter equals");

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
async fn provider_search_rejects_query_embedding_count_mismatch() {
    let store = MemoryVectorStore::new();
    let error =
        hybrid_rag_search_filtered_in_store(&store, 1, "查询", &ExtraEmbeddingProvider, 10, None)
            .await
            .expect_err("extra query embeddings should reject");

    assert!(error.to_string().contains("嵌入返回数量不匹配"));
}

#[tokio::test]
async fn caller_embedding_search_forwards_valid_vector_to_store() {
    let store = MemoryVectorStore::new();
    store
        .add_embeddings(
            "1",
            vec![embedding_record("chapter:1:100:p1")],
        )
        .await
        .expect("search record should store");

    let results = hybrid_rag_search_with_validated_embedding_in_store(
        &store,
        1,
        "chapter",
        vec![1.0, 0.0],
        10,
        None,
    )
    .await
    .expect("caller embedding search should succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "chapter:1:100:p1");
}
