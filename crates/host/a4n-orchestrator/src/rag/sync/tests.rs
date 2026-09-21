use std::collections::{HashMap, HashSet};

use a4n_domain::rag::{RAG_INDEX_FORMAT_VERSION, RAG_TEXT_INDEX_VERSION};
use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_rag_core::sync::{classify_rag_record, ensure_project_embedding_identity_matches};
use a4n_rag_core::{
    memory_store::MemoryVectorStore,
    metadata::content_hash,
    vector_store::{DeleteFilter, VectorStore},
};
use a4n_test_support::rag::embedding_record;
use serde_json::json;

use super::context::RagSyncContext;
use crate::rag::{
    chapter_rag_chunk_prefix, character_attribute_event_rag_chunk_id, character_state_rag_chunk_id,
    foreshadowing_rag_chunk_id, protagonist_rag_chunk_id, protagonist_rag_chunk_prefix,
};
use a4n_rag_core::{RagRecord, RagRecordSyncOutcome};

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
async fn chapter_prefix_filter_removes_only_the_canonical_chapter_records() {
    let store = MemoryVectorStore::new();
    let collection = "1";
    store
        .add_embeddings(
            collection,
            vec![
                embedding_record("chapter:1:100:p1"),
                embedding_record("chapter:1:100:summary"),
                embedding_record("chapter:1:100:key_event:1"),
                embedding_record("chapter:1:101:p1"),
                embedding_record("outline:1:1"),
            ],
        )
        .await
        .expect("chapter records should store");

    let filter = DeleteFilter {
        ids: Vec::new(),
        id_prefixes: vec![chapter_rag_chunk_prefix(1, 100)],
    };
    let deleted = store
        .delete(collection, filter)
        .await
        .expect("chapter records should delete");
    assert_eq!(deleted, 3);

    let remaining = store
        .metadata_by_filter(
            collection,
            DeleteFilter {
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
fn classifies_hash_index_and_embedding_identity_from_one_owner() {
    let content_hash = content_hash("旧内容").expect("content hash");
    let current = identity("current-test");
    let unchanged = json!({
        "content_hash": content_hash,
        "index_format_version": RAG_INDEX_FORMAT_VERSION,
        "text_index_version": RAG_TEXT_INDEX_VERSION,
        "embedding_backend": "LocalHash",
        "embedding_model": "current-test",
        "embedding_dimension": 2,
    });
    let old_model = json!({
        "content_hash": content_hash,
        "index_format_version": RAG_INDEX_FORMAT_VERSION,
        "text_index_version": RAG_TEXT_INDEX_VERSION,
        "embedding_backend": "LocalHash",
        "embedding_model": "old-test",
        "embedding_dimension": 2,
    });
    let old_index = json!({
        "content_hash": content_hash,
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
