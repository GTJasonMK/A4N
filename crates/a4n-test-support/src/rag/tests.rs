use a4n_core::error::AppError;
use a4n_rag_core::vector_store::VectorStore;

use super::{
    ShortWriteVectorStore, embedding_record, root_text_metadata, snapshot_text_metadata,
    test_embedding_identity,
};

#[tokio::test]
async fn short_write_store_returns_configured_count() {
    let store = ShortWriteVectorStore::new(1);
    let written = store
        .add_embeddings(
            "project",
            vec![
                embedding_record("chapter:1", "chapter", 1),
                embedding_record("chapter:2", "chapter", 2),
            ],
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
        AppError::VectorStoreError(message)
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

#[test]
fn embedding_record_preserves_identity_content_and_scope() {
    let record = embedding_record("chapter:1:9:p1", "chapter", 9);

    assert_eq!(record.id, "chapter:1:9:p1");
    assert_eq!(record.embedding, vec![1.0, 0.0]);
    assert_eq!(record.content.as_deref(), Some("chapter:1:9:p1"));
    assert_eq!(record.metadata["project_id"], serde_json::json!(1));
    assert_eq!(record.metadata["data_type"], serde_json::json!("chapter"));
    assert_eq!(record.metadata["chapter_id"], serde_json::json!(9));
    assert_eq!(record.metadata["chapter_number"], serde_json::json!(3));
}

#[test]
fn project_family_metadata_fixtures_preserve_root_and_snapshot_scope() {
    let root = root_text_metadata(7);
    let snapshot = snapshot_text_metadata(7, 11, 13, 17, "chapter");

    assert_eq!(root["root_project_id"], serde_json::json!(7));
    assert_eq!(root["source_snapshot_id"], serde_json::Value::Null);
    assert_eq!(root["source_chapter_id"], serde_json::Value::Null);
    assert_eq!(root["source_snapshot_chapter_id"], serde_json::Value::Null);
    assert_eq!(root["derived_project_id"], serde_json::Value::Null);

    assert_eq!(snapshot["root_project_id"], serde_json::json!(7));
    assert_eq!(snapshot["source_snapshot_id"], serde_json::json!(11));
    assert_eq!(snapshot["source_chapter_id"], serde_json::json!(13));
    assert_eq!(
        snapshot["source_snapshot_chapter_id"],
        serde_json::json!(17)
    );
    assert_eq!(snapshot["derived_project_id"], serde_json::Value::Null);
    assert_eq!(snapshot["data_type"], serde_json::json!("chapter"));
}
