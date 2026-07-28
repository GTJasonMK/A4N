use super::*;
use crate::event::progress::ProgressPhaseKind;
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
    assert_eq!(progress.percent() * 100.0, service_event.percent);
    assert_eq!(
        serde_json::to_value(&progress.meta).unwrap(),
        serde_json::to_value(&service_event.meta).unwrap()
    );
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
    let handle = manager.register(test_handle());

    assert!(manager.cancel("task-1"));
    assert!(handle.is_cancelled());
    assert!(manager.list_active_snapshots().is_empty());
}

fn test_handle() -> TaskHandle {
    TaskHandle {
        task_id: "task-1".to_string(),
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
