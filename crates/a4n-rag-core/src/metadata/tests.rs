use a4n_core::rag::RAG_TEXT_INDEX_VERSION;
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
fn metadata_normalization_uses_argument_project_and_embedding_identity() {
    let metadata = normalize_metadata(
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
        &identity(),
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
    assert!(metadata_matches_current_text_index(&metadata));
    assert!(metadata_matches_embedding_identity(&metadata, &identity()));
    assert_eq!(
        metadata_content_hash(&metadata),
        metadata["content_hash"].as_str()
    );
}

#[test]
fn metadata_normalization_rejects_missing_and_cross_project_family_context() {
    let missing = normalize_metadata(
        42,
        "正文内容",
        serde_json::json!({"data_type": "chapter"}),
        &identity(),
    )
    .expect_err("missing family metadata should reject");
    assert!(missing.to_string().contains("root_project_id"));

    let cross_project = normalize_metadata(
        8,
        "正文内容",
        serde_json::json!({
            "root_project_id": 1,
            "source_snapshot_id": 2,
            "source_chapter_id": 3,
            "source_snapshot_chapter_id": 4,
            "derived_project_id": 9,
        }),
        &identity(),
    )
    .expect_err("derived project mismatch should reject");
    assert!(
        cross_project
            .to_string()
            .contains("derived_project_id 必须等于当前项目")
    );

    let blank = normalize_metadata(
        42,
        "  ",
        serde_json::json!({
            "root_project_id": 42,
            "source_snapshot_id": null,
            "source_chapter_id": null,
            "source_snapshot_chapter_id": null,
            "derived_project_id": null,
        }),
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
