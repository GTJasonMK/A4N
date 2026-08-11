use std::collections::{HashMap, HashSet};

use a4n_core::rag::{RAG_INDEX_FORMAT_VERSION, RAG_TEXT_INDEX_VERSION};
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::{
    memory_store::MemoryVectorStore,
    metadata::content_hash,
    vector_store::{DeleteFilter, VectorStore},
};
use a4n_test_support::rag::embedding_record;
use serde_json::json;

use super::context::RagSyncContext;
use super::identity::{classify_rag_record, ensure_project_embedding_identity_matches};
use super::keys::{
    chapter_rag_chunk_prefix, chapter_rag_delete_filter, character_attribute_event_rag_chunk_id,
    character_state_rag_chunk_id, foreshadowing_rag_chunk_id, protagonist_rag_chunk_id,
    protagonist_rag_chunk_prefix,
};
use super::metadata::root_text_family_metadata;
use super::{RagRecord, RagRecordSyncOutcome};

fn identity(model: &str) -> EmbeddingModelIdentity {
    EmbeddingModelIdentity {
        backend: "LocalHash".to_string(),
        model: model.to_string(),
        dimension: 2,
    }
}

#[test]
fn canonical_character_and_foreshadowing_ids_share_write_delete_identity() {
    assert_eq!(chapter_rag_chunk_prefix(7, 10), "chapter:7:10:");
    assert_eq!(character_state_rag_chunk_id(7, 11), "character_state:7:11");
    assert_eq!(
        character_attribute_event_rag_chunk_id(7, 12),
        "character_attribute_event:7:12"
    );
    assert_eq!(foreshadowing_rag_chunk_id(7, 13), "foreshadowing:7:13");
    assert_eq!(protagonist_rag_chunk_prefix(7, 14), "protagonist:7:14:");
    assert_eq!(
        protagonist_rag_chunk_id(7, 14, "current_state"),
        "protagonist:7:14:current_state"
    );
}

#[tokio::test]
async fn chapter_delete_filter_removes_only_the_canonical_chapter_prefix() {
    let store = MemoryVectorStore::new();
    let collection = "1";
    store
        .add_embeddings(
            collection,
            vec![
                embedding_record("chapter:1:100:p1", "chapter", 100),
                embedding_record("chapter:1:100:summary", "chapter_summary", 100),
                embedding_record("chapter:1:100:key_event:1", "key_event", 100),
                embedding_record("chapter:1:101:p1", "chapter", 101),
                embedding_record("outline:1:1", "outline", 0),
            ],
        )
        .await
        .expect("chapter records should store");

    let filter = chapter_rag_delete_filter(1, 100, 3).expect("valid chapter delete filter");
    let deleted = store
        .delete(collection, filter)
        .await
        .expect("chapter records should delete");
    assert_eq!(deleted, 3);

    let remaining = store
        .metadata_by_filter(
            collection,
            DeleteFilter {
                project_id: 1,
                data_type: None,
                chapter_number: None,
                ids: Vec::new(),
                id_prefixes: Vec::new(),
            },
        )
        .await
        .expect("remaining metadata should load");
    let remaining_ids = remaining.keys().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(remaining_ids.len(), 2);
    assert!(remaining_ids.contains(&"chapter:1:101:p1"));
    assert!(remaining_ids.contains(&"outline:1:1"));
}

#[test]
fn chapter_delete_filter_rejects_non_positive_chapter_number() {
    let error = chapter_rag_delete_filter(1, 100, 0)
        .expect_err("non-positive chapter number should reject");
    assert_eq!(
        error.to_string(),
        "参数无效：RAG 章节清理 chapter_number 必须大于 0，实际为 0"
    );
}

#[test]
fn classifies_hash_index_and_embedding_identity_from_one_owner() {
    let content_hash = content_hash("旧内容").expect("content hash");
    let current = identity("current-test");
    let unchanged = json!({
        "content_hash": content_hash,
        "source_generation": content_hash,
        "index_format_version": RAG_INDEX_FORMAT_VERSION,
        "text_index_version": RAG_TEXT_INDEX_VERSION,
        "embedding_backend": "LocalHash",
        "embedding_model": "current-test",
        "embedding_dimension": 2,
    });
    let old_model = json!({
        "content_hash": content_hash,
        "source_generation": content_hash,
        "index_format_version": RAG_INDEX_FORMAT_VERSION,
        "text_index_version": RAG_TEXT_INDEX_VERSION,
        "embedding_backend": "LocalHash",
        "embedding_model": "old-test",
        "embedding_dimension": 2,
    });
    let old_index = json!({
        "content_hash": content_hash,
        "source_generation": content_hash,
        "index_format_version": RAG_INDEX_FORMAT_VERSION,
        "text_index_version": "legacy",
        "embedding_backend": "LocalHash",
        "embedding_model": "current-test",
        "embedding_dimension": 2,
    });

    assert_eq!(
        classify_rag_record(Some(&unchanged), &content_hash, &current),
        RagRecordSyncOutcome::Skipped
    );
    assert_eq!(
        classify_rag_record(Some(&old_model), &content_hash, &current),
        RagRecordSyncOutcome::Updated
    );
    assert_eq!(
        classify_rag_record(Some(&old_index), &content_hash, &current),
        RagRecordSyncOutcome::Updated
    );
    assert_eq!(
        classify_rag_record(
            Some(&json!({"content_hash": "different"})),
            &content_hash,
            &current
        ),
        RagRecordSyncOutcome::Updated
    );
    assert_eq!(
        classify_rag_record(None, &content_hash, &current),
        RagRecordSyncOutcome::Added
    );
}

#[test]
fn preparing_record_removes_seen_chunk_from_stale_cleanup_scope() {
    let content_hash = content_hash("旧内容").expect("content hash");
    let mut context = RagSyncContext {
        project_id: 1,
        embedding_identity: identity("current-test"),
        stored_metadata: HashMap::from([(
            "chunk:seen".to_string(),
            json!({
                "content_hash": content_hash,
                "source_generation": content_hash,
                "index_format_version": RAG_INDEX_FORMAT_VERSION,
                "text_index_version": RAG_TEXT_INDEX_VERSION,
                "embedding_backend": "LocalHash",
                "embedding_model": "current-test",
                "embedding_dimension": 2,
            }),
        )]),
        stale_ids: HashSet::from(["chunk:seen".to_string(), "chunk:stale".to_string()]),
    };

    let prepared = context
        .prepare_record(RagRecord {
            chunk_id: "chunk:seen".to_string(),
            text: "旧内容".to_string(),
            metadata: json!({}),
        })
        .expect("seen record should prepare");

    assert_eq!(prepared.outcome(), RagRecordSyncOutcome::Skipped);
    assert!(!context.stale_ids.contains("chunk:seen"));
    assert!(context.stale_ids.contains("chunk:stale"));
}

#[test]
fn root_text_metadata_is_inserted_and_conflicts_are_rejected() {
    let metadata =
        root_text_family_metadata(7, json!({"chapter_id": 9})).expect("root text metadata");
    assert_eq!(metadata["root_project_id"], 7);
    assert_eq!(metadata["source_chapter_id"], 9);
    assert!(metadata["source_snapshot_id"].is_null());
    assert!(metadata["source_snapshot_chapter_id"].is_null());
    assert!(metadata["derived_project_id"].is_null());

    let error = root_text_family_metadata(7, json!({"root_project_id": 8}))
        .expect_err("conflicting root project must fail");
    assert!(error.to_string().contains("expected=7, actual=8"));

    for chapter_id in [json!(0), json!(-1), json!("9"), json!(null)] {
        let error = root_text_family_metadata(7, json!({"chapter_id": chapter_id}))
            .expect_err("invalid present chapter id must fail");
        assert!(error.to_string().contains("chapter_id 必须是正整数"));
    }
}

#[test]
fn project_embedding_identity_lock_reports_actual_and_expected() {
    let stored = HashMap::from([(
        "chunk:1".to_string(),
        json!({
            "embedding_backend": "LocalHash",
            "embedding_model": "old-test",
            "embedding_dimension": 2,
        }),
    )]);

    let error = ensure_project_embedding_identity_matches(&stored, &identity("current-test"))
        .expect_err("different project embedding identity must fail");

    assert!(error.to_string().contains("old-test"));
    assert!(error.to_string().contains("current-test"));
    assert!(error.to_string().contains("显式重建 RAG"));
}
