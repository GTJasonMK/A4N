use super::super::sync::{
    RagIngestProgress, RagRecord, RagRecordSyncOutcome, RagSyncContext, required_record_dimension,
};
use super::test_embedding_identity;
use a4n_core::rag::RAG_TEXT_INDEX_VERSION;
use a4n_services::rag::RagService;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

#[test]
fn rag_ingest_progress_reports_stage_and_advances() {
    let events = Arc::new(Mutex::new(Vec::<(u32, u32, String)>::new()));
    let captured = events.clone();
    let reporter = move |current: u32, total: u32, message: &str| {
        captured
            .lock()
            .expect("progress events lock")
            .push((current, total, message.to_string()));
    };
    let mut progress = RagIngestProgress::new(3, Some(&reporter), None);

    progress.stage("准备同步").expect("stage reports");
    progress.advance(2, "已同步 2 条").expect("advance reports");
    progress
        .advance(9, "同步完成")
        .expect("advance caps at total");

    let events = events.lock().expect("progress events lock");
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], (0, 3, "准备同步".to_string()));
    assert_eq!(events[1], (2, 3, "已同步 2 条".to_string()));
    assert_eq!(events[2], (3, 3, "同步完成".to_string()));
}

#[test]
fn rag_ingest_progress_rejects_after_cancellation() {
    let cancel_token = CancellationToken::new();
    cancel_token.cancel();
    let progress = RagIngestProgress::new(1, None, Some(&cancel_token));

    let err = progress
        .stage("不应继续同步")
        .expect_err("cancelled progress should fail");

    assert!(err.contains("RAG 同步已停止"));
}

#[test]
fn rag_sync_context_classifies_unchanged_changed_and_new_records() {
    let unchanged_hash = RagService::content_hash("旧内容").expect("hash");
    let mut sync = RagSyncContext {
        project_id: 1,
        embedding_identity: test_embedding_identity("current-test"),
        stored_metadata: std::collections::HashMap::from([
            (
                "chunk:unchanged".to_string(),
                json!({
                    "content_hash": unchanged_hash,
                    "text_index_version": RAG_TEXT_INDEX_VERSION,
                    "embedding_backend": "LocalHash",
                    "embedding_model": "current-test",
                    "embedding_dimension": 2,
                }),
            ),
            (
                "chunk:old-model".to_string(),
                json!({
                    "content_hash": unchanged_hash,
                    "text_index_version": RAG_TEXT_INDEX_VERSION,
                    "embedding_backend": "LocalHash",
                    "embedding_model": "old-test",
                    "embedding_dimension": 2,
                }),
            ),
            (
                "chunk:old-index".to_string(),
                json!({
                    "content_hash": unchanged_hash,
                    "text_index_version": "legacy",
                    "embedding_backend": "LocalHash",
                    "embedding_model": "current-test",
                    "embedding_dimension": 2,
                }),
            ),
            (
                "chunk:changed".to_string(),
                json!({"content_hash": "different"}),
            ),
        ]),
        stale_ids: std::collections::HashSet::from([
            "chunk:unchanged".to_string(),
            "chunk:old-model".to_string(),
            "chunk:old-index".to_string(),
            "chunk:changed".to_string(),
            "chunk:stale".to_string(),
        ]),
    };

    assert_eq!(
        sync.classify_record_hash(
            "chunk:unchanged",
            &RagService::content_hash("旧内容").expect("hash")
        ),
        RagRecordSyncOutcome::Skipped
    );
    assert_eq!(
        sync.classify_record_hash(
            "chunk:old-model",
            &RagService::content_hash("旧内容").expect("hash")
        ),
        RagRecordSyncOutcome::Updated
    );
    assert_eq!(
        sync.classify_record_hash(
            "chunk:old-index",
            &RagService::content_hash("旧内容").expect("hash")
        ),
        RagRecordSyncOutcome::Updated
    );
    assert_eq!(
        sync.classify_record_hash(
            "chunk:changed",
            &RagService::content_hash("新内容").expect("hash")
        ),
        RagRecordSyncOutcome::Updated
    );
    assert_eq!(
        sync.classify_record_hash(
            "chunk:new",
            &RagService::content_hash("新增").expect("hash")
        ),
        RagRecordSyncOutcome::Added
    );
    assert!(!sync.stale_ids.contains("chunk:unchanged"));
    assert!(!sync.stale_ids.contains("chunk:old-model"));
    assert!(!sync.stale_ids.contains("chunk:old-index"));
    assert!(!sync.stale_ids.contains("chunk:changed"));
    assert!(sync.stale_ids.contains("chunk:stale"));
}

#[test]
fn required_record_dimension_rejects_missing_or_blank_metadata() {
    let valid = RagRecord {
        chunk_id: "chunk:valid".to_string(),
        text: "正文".to_string(),
        metadata: json!({ "dimension": " 背景 " }),
    };
    assert_eq!(
        required_record_dimension(&valid, "有效记录").expect("dimension should trim"),
        "背景"
    );

    let missing = RagRecord {
        chunk_id: "chunk:missing".to_string(),
        text: "正文".to_string(),
        metadata: json!({}),
    };
    let err = required_record_dimension(&missing, "缺失记录")
        .expect_err("missing dimension should reject");
    assert!(err.contains("缺失记录 缺少 dimension 元数据"));

    let blank = RagRecord {
        chunk_id: "chunk:blank".to_string(),
        text: "正文".to_string(),
        metadata: json!({ "dimension": "  " }),
    };
    let err =
        required_record_dimension(&blank, "空维度记录").expect_err("blank dimension should reject");
    assert!(err.contains("空维度记录 缺少 dimension 元数据"));
}
