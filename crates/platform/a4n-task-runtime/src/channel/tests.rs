use super::*;
use a4n_progress::ProgressPhaseKind;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const ALL_TASK_KINDS: [TaskKind; 14] = [
    TaskKind::LlmCall,
    TaskKind::ImageGeneration,
    TaskKind::VideoGeneration,
    TaskKind::GalgameGeneration,
    TaskKind::RagIngestion,
    TaskKind::ExportGeneration,
    TaskKind::EmbeddingDownload,
    TaskKind::VideoPoll,
    TaskKind::SourceSync,
    TaskKind::Assembly,
    TaskKind::EmbeddingCall,
    TaskKind::ImportAnalysis,
    TaskKind::MangaGeneration,
    TaskKind::TextGeneration,
];

#[test]
fn task_kind_storage_and_serde_round_trip_cover_every_variant() {
    for task_kind in ALL_TASK_KINDS {
        let storage = task_kind.as_str();
        assert_eq!(TaskKind::parse(storage), Some(task_kind));
        assert_eq!(TaskKind::parse_storage(storage).unwrap(), task_kind);
        assert_eq!(storage.parse::<TaskKind>().unwrap(), task_kind);

        let json = serde_json::to_string(&task_kind).unwrap();
        assert_eq!(serde_json::from_str::<TaskKind>(&json).unwrap(), task_kind);
    }
}

#[test]
fn task_kind_rejects_unknown_storage_value() {
    assert_eq!(TaskKind::parse("unknown_task_kind"), None);
    let error = TaskKind::parse_storage("unknown_task_kind").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("未知任务类型: unknown_task_kind")
    );
}

#[test]
fn task_kind_progress_meta_has_a_stable_stage_for_every_variant() {
    let expected = [
        "llm_call",
        "image_generation",
        "video_generation",
        "galgame_pipeline",
        "rag_ingestion",
        "export_generation",
        "embedding_download",
        "video_poll",
        "source_sync",
        "assembly",
        "embedding_call",
        "import_analysis",
        "manga_generation",
        "text_generation",
    ];

    for (task_kind, expected_stage) in ALL_TASK_KINDS.into_iter().zip(expected) {
        let meta = task_kind.progress_meta("detail");
        assert_eq!(meta.stage_code, expected_stage);
        assert!(!meta.stage_label.is_empty());
        assert_eq!(meta.detail.as_deref(), Some("detail"));
    }
}

#[test]
fn galgame_task_type_uses_galgame_generation_progress_meta() {
    let meta = TaskKind::GalgameGeneration.progress_meta("正在执行 Galgame 管线");

    assert_eq!(TaskKind::GalgameGeneration.as_str(), "galgame_generation");
    assert_eq!(meta.task_kind, "galgame_generation");
    assert_eq!(meta.stage_code, "galgame_pipeline");
    assert_eq!(meta.stage_label, "Galgame 管线");
    assert!(matches!(&meta.phase_kind, ProgressPhaseKind::ModelWait));
    assert_eq!(meta.detail.as_deref(), Some("正在执行 Galgame 管线"));
}

#[test]
fn task_progress_updates_snapshot_and_history() {
    let handle = test_handle();

    handle.update_progress(3, 10, "生成中");
    let progress = handle.progress.read();
    let service_event = handle.progress_events.read().back().cloned().unwrap();

    assert_eq!(progress.current, service_event.current);
    assert_eq!(progress.total, service_event.total);
    assert_eq!(progress.message, service_event.message);
    assert_eq!(
        serde_json::to_value(&progress.meta).unwrap(),
        serde_json::to_value(&service_event.meta).unwrap()
    );
}

#[test]
fn progress_event_cursor_reads_new_events_and_resets_when_expired() {
    let handle = test_handle();
    let events = (0..200)
        .map(|index| TaskProgressEvent {
            id: format!("task-1-{index}"),
            current: index,
            total: 200,
            message: format!("progress {index}"),
            meta: None,
            timestamp: "2026-10-11T00:00:00Z".into(),
        })
        .collect::<VecDeque<_>>();
    *handle.progress_events.write() = events;

    let incremental = handle.progress_events_after(Some("task-1-198"), 200);
    assert_eq!(
        incremental
            .iter()
            .map(|event| event.id.as_str())
            .collect::<Vec<_>>(),
        ["task-1-199"]
    );

    let expired = handle.progress_events_after(Some("task-1-expired"), 200);
    assert_eq!(expired.len(), 200);
    assert_eq!(expired.first().unwrap().id, "task-1-0");
    assert_eq!(expired.last().unwrap().id, "task-1-199");
}

#[test]
#[ignore = "manual performance benchmark; exercises the production progress ring limit"]
fn benchmark_progress_event_cursor_projection() {
    const EVENT_COUNT: usize = 200;
    const MESSAGE_LENGTH: usize = 256;
    const SAMPLES: usize = 15;
    const CURSOR: &str = "task-1-000198";

    fn response_bytes(events: &[TaskProgressEvent]) -> usize {
        serde_json::to_vec(&serde_json::json!({
            "events": events,
            "stream_text": "x".repeat(512),
        }))
        .unwrap()
        .len()
    }

    fn measure(handle: &TaskHandle, cursor: Option<&str>) -> (f64, usize) {
        let started = std::time::Instant::now();
        let events = handle.progress_events_after(cursor, EVENT_COUNT);
        let bytes = response_bytes(&events);
        (started.elapsed().as_secs_f64() * 1000.0, bytes)
    }

    fn percentile(samples: &[f64], percentile: f64) -> f64 {
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        sorted[((sorted.len() - 1) as f64 * percentile).round() as usize]
    }

    let handle = test_handle();
    let events = (0..EVENT_COUNT)
        .map(|index| TaskProgressEvent {
            id: format!("task-1-{index:06}"),
            current: index as u32,
            total: EVENT_COUNT as u32,
            message: "x".repeat(MESSAGE_LENGTH),
            meta: None,
            timestamp: "2026-10-11T00:00:00Z".into(),
        })
        .collect::<VecDeque<_>>();
    *handle.progress_events.write() = events;

    let full = handle.progress_events_after(None, EVENT_COUNT);
    let incremental = handle.progress_events_after(Some(CURSOR), EVENT_COUNT);
    assert_eq!(full.len(), EVENT_COUNT);
    assert_eq!(incremental.len(), 1);
    let full_bytes = response_bytes(&full);
    let incremental_bytes = response_bytes(&incremental);

    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut incremental_samples = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            full_samples.push(measure(&handle, None).0);
            incremental_samples.push(measure(&handle, Some(CURSOR)).0);
        } else {
            incremental_samples.push(measure(&handle, Some(CURSOR)).0);
            full_samples.push(measure(&handle, None).0);
        }
    }

    let full_p50 = percentile(&full_samples, 0.50);
    let full_p95 = percentile(&full_samples, 0.95);
    let incremental_p50 = percentile(&incremental_samples, 0.50);
    let incremental_p95 = percentile(&incremental_samples, 0.95);
    let improvement = (1.0 - incremental_p50 / full_p50) * 100.0;
    println!(
        "progress_event_cursor events={EVENT_COUNT} message_length={MESSAGE_LENGTH} full_bytes={full_bytes} incremental_bytes={incremental_bytes} full_p50_p95_ms={full_p50:.3}/{full_p95:.3} incremental_p50_p95_ms={incremental_p50:.3}/{incremental_p95:.3} p50_improvement={improvement:.1}%"
    );
    assert!(incremental_bytes < full_bytes / 2);
    assert!(incremental_p50 < full_p50 * 0.7);
}

#[test]
fn stream_text_updates_snapshot_without_polluting_event_history() {
    let handle = test_handle();
    handle.update_progress(3, 10, "生成中");
    let events_after_progress = handle.progress_events.read().len();

    // delta 频率是每 token 一次；推成事件会冲爆环形缓冲并挤掉字段级里程碑。
    for _ in 0..500 {
        handle.update_stream_text("{\"scenes\":[{\"name\":\"庭院");
    }

    assert_eq!(
        handle.progress_events.read().len(),
        events_after_progress,
        "活体信号不得进入事件历史"
    );
    assert_eq!(
        handle.progress.read().stream_text.as_deref(),
        Some("{\"scenes\":[{\"name\":\"庭院"),
        "快照应保留最近一次尾巴"
    );
}

#[test]
fn stream_text_survives_later_milestones() {
    let handle = test_handle();
    handle.update_stream_text("{\"scenes\":[{");
    // 阶段推进不应让流式尾巴闪断。
    handle.update_progress(5, 10, "第 1 个场景：名称已完成");

    assert_eq!(
        handle.progress.read().stream_text.as_deref(),
        Some("{\"scenes\":[{")
    );
}

#[test]
fn stream_text_keeps_only_the_tail_on_utf8_boundaries() {
    let handle = test_handle();
    let long_text: String = "庭院深深".repeat(400);
    handle.update_stream_text(&long_text);

    let progress = handle.progress.read();
    let kept = progress.stream_text.as_deref().expect("应保留尾部");
    assert!(kept.chars().count() <= 512);
    assert!(long_text.ends_with(kept), "应保留尾部而非头部");
}

#[test]
fn task_byte_progress_keeps_host_bytes_and_records_scaled_progress() {
    let handle = test_handle();

    handle.update_progress_bytes(512, 1024, 4, "下载中");
    let progress = handle.progress.read();
    assert_eq!(progress.current, 2);
    assert_eq!(progress.total, 4);
    assert_eq!(
        progress.meta.as_ref().unwrap().unit_label.as_deref(),
        Some("字节")
    );
}

#[test]
fn task_manager_cancels_the_registered_task_identity() {
    let manager = TaskManager::new();
    let handle = manager.register(test_handle()).unwrap();

    assert!(manager.cancel("task-1"));
    assert!(handle.is_cancelled());
    assert!(manager.list_active_snapshots().is_empty());
}

#[test]
fn cancelled_task_ignores_late_progress_updates() {
    let handle = test_handle();
    handle.update_progress(3, 10, "生成中");
    handle.cancel();
    handle.update_progress(8, 10, "迟到进度");

    assert_eq!(handle.progress.read().current, 3);
    assert_eq!(handle.progress_events.read().len(), 1);
}

#[test]
fn task_manager_applies_only_the_owners_pre_registration_cancel() {
    let manager = TaskManager::new();
    assert!(!manager.cancel_or_remember("task-1", 7));

    let handle = manager.register(test_handle()).unwrap();
    assert!(handle.is_cancelled());

    assert!(!manager.cancel_or_remember("task-2", 8));
    let mut other = test_handle();
    other.task_id = "task-2".to_string();
    let other = manager.register(other).unwrap();
    assert!(!other.is_cancelled());
}

#[test]
fn task_manager_rejects_duplicate_ids_without_replacing_the_owner() {
    let manager = TaskManager::new();
    let first = manager.register(test_handle()).unwrap();

    assert!(manager.register(test_handle()).is_err());
    assert!(Arc::ptr_eq(&manager.get("task-1").unwrap(), &first));
}

#[test]
fn task_manager_keeps_pre_registration_cancellations_separate_by_owner() {
    let manager = TaskManager::new();
    assert!(!manager.cancel_or_remember("task-1", 7));
    assert!(!manager.cancel_or_remember("task-1", 8));

    let handle = manager.register(test_handle()).unwrap();
    assert!(handle.is_cancelled());

    assert!(manager.remove(&handle).is_ok());
    let mut other = test_handle();
    other.owner_user_id = Some(8);
    let other = manager.register(other).unwrap();
    assert!(!other.is_cancelled());
}

#[test]
fn stale_handle_cannot_remove_a_new_registration_with_the_same_id() {
    let manager = TaskManager::new();
    let first = manager.register(test_handle()).unwrap();
    assert!(manager.remove(&first).is_ok());

    let second = manager.register(test_handle()).unwrap();
    assert!(manager.remove(&first).is_err());
    assert!(Arc::ptr_eq(&manager.get("task-1").unwrap(), &second));
}

#[test]
fn task_id_normalization_has_one_strict_contract() {
    assert_eq!(normalize_task_id(" task-1 ").unwrap(), "task-1");
    for task_id in ["", "task/path", "任务"] {
        assert!(normalize_task_id(task_id).is_err());
    }
    assert!(normalize_task_id(&"a".repeat(161)).is_err());
}

fn test_handle() -> TaskHandle {
    TaskHandle {
        task_id: "task-1".to_string(),
        owner_user_id: Some(7),
        cancel_token: CancellationToken::new(),
        progress: Arc::new(RwLock::new(TaskProgress::with_meta(0, 10, "准备", None))),
        progress_events: Arc::new(RwLock::new(VecDeque::new())),
        task_type: TaskKind::VideoGeneration,
        project_id: Some(1),
        root_project_id: None,
        source_snapshot_id: None,
        task_scope: None,
    }
}
