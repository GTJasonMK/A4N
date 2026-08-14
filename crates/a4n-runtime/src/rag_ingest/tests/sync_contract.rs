use super::super::sync::{ExpectedSyncStats, RagIngestProgress, RagRecordSyncOutcome};
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
    let error = progress
        .advance(2, "不应静默截断")
        .expect_err("progress beyond the declared total must fail");
    assert!(error.contains("超过声明总量"));
    progress.advance(1, "同步完成").expect("reaches total");

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
fn rag_ingest_progress_rejects_u32_overflow() {
    let mut progress = RagIngestProgress::new(u32::MAX, None, None);
    progress
        .advance(u32::MAX, "推进到上限")
        .expect("first advance reaches the declared total");

    let err = progress
        .advance(1, "不应饱和")
        .expect_err("progress overflow must be visible");

    assert!(err.contains("RAG 同步进度超过 u32 上限"));
}

#[test]
fn rag_sync_stats_reject_outcome_count_overflow_without_partial_update() {
    let mut stats = ExpectedSyncStats {
        added_count: u32::MAX,
        ..Default::default()
    };

    let err = stats
        .record_outcome(RagRecordSyncOutcome::Added)
        .expect_err("outcome overflow must be visible");

    assert!(err.contains("新增记录数超过 u32 上限"));
    assert_eq!(stats.added_count, u32::MAX);
    assert_eq!(stats.chunk_count, 0);
}

#[test]
fn rag_sync_stats_reject_deleted_count_overflow_without_partial_update() {
    let mut stats = ExpectedSyncStats {
        deleted_count: u32::MAX,
        ..Default::default()
    };

    let err = stats
        .add_deleted(1)
        .expect_err("deleted count overflow must be visible");

    assert!(err.contains("删除记录数超过 u32 上限"));
    assert_eq!(stats.deleted_count, u32::MAX);
}
