use super::*;
use std::sync::{Arc, Mutex};

#[test]
fn storage_enums_round_trip_and_reject_unknown_values() {
    for (priority, storage) in [
        (TaskPriority::High, "high"),
        (TaskPriority::Medium, "medium"),
        (TaskPriority::Low, "low"),
    ] {
        assert_eq!(priority.as_str(), storage);
        assert_eq!(TaskPriority::parse(storage), Some(priority));
        assert_eq!(TaskPriority::parse_storage(storage).unwrap(), priority);
    }

    for (task_kind, storage) in [
        (TaskKind::LlmCall, "llm_call"),
        (TaskKind::ImageGeneration, "image_generation"),
        (TaskKind::VideoGeneration, "video_generation"),
        (TaskKind::VideoPoll, "video_poll"),
        (TaskKind::SourceSync, "source_sync"),
        (TaskKind::Assembly, "assembly"),
        (TaskKind::EmbeddingCall, "embedding_call"),
        (TaskKind::RagIngestion, "rag_ingestion"),
        (TaskKind::ImportAnalysis, "import_analysis"),
        (TaskKind::ExportGeneration, "export_generation"),
        (TaskKind::EmbeddingDownload, "embedding_download"),
        (TaskKind::GalgameGeneration, "galgame_generation"),
        (TaskKind::MangaGeneration, "manga_generation"),
        (TaskKind::TextGeneration, "text_generation"),
    ] {
        assert_eq!(task_kind.as_str(), storage);
        assert_eq!(TaskKind::parse(storage), Some(task_kind));
        assert_eq!(TaskKind::parse_storage(storage).unwrap(), task_kind);
    }

    assert_eq!(
        TaskPriority::parse_storage("urgent")
            .expect_err("unknown priority must fail")
            .to_string(),
        "参数无效：未知任务优先级: urgent"
    );
    assert_eq!(
        TaskKind::parse_storage("render")
            .expect_err("unknown task type must fail")
            .to_string(),
        "参数无效：未知任务类型: render"
    );
}

#[test]
fn task_status_storage_keeps_terminal_payload_contracts() {
    assert!(matches!(
        TaskStatus::parse_storage("pending", None).unwrap(),
        TaskStatus::Pending
    ));
    assert!(matches!(
        TaskStatus::parse_storage("interrupted", Some("host restarted")).unwrap(),
        TaskStatus::Interrupted { reason: Some(reason) } if reason == "host restarted"
    ));
    assert!(matches!(
        TaskStatus::parse_storage("failed", Some("provider failed")).unwrap(),
        TaskStatus::Failed { error } if error == "provider failed"
    ));
    assert_eq!(
        TaskStatus::parse_storage("failed", None)
            .expect_err("failed status requires error")
            .to_string(),
        "参数无效：失败任务缺少 error_message"
    );
    assert_eq!(
        TaskStatus::parse_storage("stale", None)
            .expect_err("unknown status must fail")
            .to_string(),
        "参数无效：未知任务状态: stale"
    );
}

#[test]
fn task_permit_reports_complete_fail_and_implicit_drop() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let record = |events: Arc<Mutex<Vec<TaskStatus>>>| {
        Box::new(move |status| events.lock().unwrap().push(status))
            as Box<dyn FnOnce(TaskStatus) + Send>
    };

    let completed = TaskPermit::new(
        "complete-task".to_string(),
        TaskKind::LlmCall,
        TaskPriority::High,
        record(events.clone()),
    );
    assert_eq!(completed.task_id(), "complete-task");
    assert_eq!(completed.task_type(), &TaskKind::LlmCall);
    assert_eq!(completed.priority(), TaskPriority::High);
    completed.complete();

    TaskPermit::new(
        "failed-task".to_string(),
        TaskKind::ImageGeneration,
        TaskPriority::Medium,
        record(events.clone()),
    )
    .fail("provider failed".to_string());

    drop(TaskPermit::new(
        "dropped-task".to_string(),
        TaskKind::RagIngestion,
        TaskPriority::Low,
        record(events.clone()),
    ));

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 3);
    assert!(matches!(events[0], TaskStatus::Completed));
    assert!(matches!(
        &events[1],
        TaskStatus::Failed { error } if error == "provider failed"
    ));
    assert!(matches!(
        &events[2],
        TaskStatus::Failed { error } if error == "任务许可被释放但未显式完成"
    ));
}
