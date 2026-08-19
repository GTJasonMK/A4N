use a4n_core::rag::{RAG_INDEX_FORMAT_VERSION, RAG_TEXT_INDEX_VERSION};
use a4n_embedding::provider::EmbeddingModelIdentity;

use super::*;

fn identity() -> EmbeddingModelIdentity {
    EmbeddingModelIdentity {
        backend: "LocalHash".to_string(),
        model: "matching-test".to_string(),
        dimension: 2,
    }
}

#[test]
fn metadata_normalization_adds_hash_index_and_embedding_identity() {
    let metadata = normalize_metadata(
        "正文内容",
        serde_json::json!({
            "data_type": "chapter",
        }),
        &identity(),
    )
    .expect("valid metadata");

    assert_eq!(metadata["data_type"], serde_json::json!("chapter"));
    assert_eq!(
        metadata["index_format_version"],
        serde_json::json!(RAG_INDEX_FORMAT_VERSION)
    );
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
    assert!(metadata_matches_current_text_index(&metadata));
    assert!(metadata_matches_current_index_format(&metadata));
    assert!(metadata_matches_embedding_identity(&metadata, &identity()));
    assert_eq!(
        metadata_content_hash(&metadata),
        metadata["content_hash"].as_str()
    );
}

#[test]
fn metadata_normalization_rejects_non_object_and_blank_content() {
    let non_object = normalize_metadata("正文内容", serde_json::json!([]), &identity())
        .expect_err("non-object metadata should reject");
    assert!(non_object.to_string().contains("必须是 JSON 对象"));

    let blank = normalize_metadata(
        "  ",
        serde_json::json!({"data_type": "chapter"}),
        &identity(),
    )
    .expect_err("blank metadata content should reject");
    assert!(blank.to_string().contains("RAG content 不能为空"));
}

#[test]
fn content_hash_trims_input_and_rejects_blank_content() {
    assert_eq!(
        content_hash("  正文  ").expect("content hash"),
        content_hash("正文").expect("trimmed content hash")
    );
    let error = content_hash("  ").expect_err("blank content should reject");
    assert!(error.to_string().contains("RAG content 不能为空"));
}
