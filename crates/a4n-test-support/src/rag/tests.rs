use a4n_domain::ProviderError as AppError;
use a4n_rag_core::vector_store::VectorStore;

use super::{ShortWriteVectorStore, embedding_record, test_embedding_identity};

#[tokio::test]
async fn short_write_store_returns_configured_count() {
    let store = ShortWriteVectorStore::new(1);
    let written = store
        .add_embeddings(
            "project",
            vec![embedding_record("chapter:1"), embedding_record("chapter:2")],
        )
        .await
        .expect("configured short write");

    assert_eq!(written, 1);
}

#[tokio::test]
async fn short_write_store_rejects_non_write_operations() {
    let store = ShortWriteVectorStore::new(1);
    let error = store
        .query("project", vec![1.0, 0.0], 1, None)
        .await
        .expect_err("query is not supported by the short-write double");

    assert!(matches!(
        error,
        AppError::VectorStore(message)
            if message == "unexpected ShortWriteVectorStore method call"
    ));
}

#[test]
fn embedding_identity_uses_local_hash_contract() {
    let identity = test_embedding_identity("fixture-model");

    assert_eq!(identity.backend, "LocalHash");
    assert_eq!(identity.model, "fixture-model");
    assert_eq!(identity.dimension, 2);
}
