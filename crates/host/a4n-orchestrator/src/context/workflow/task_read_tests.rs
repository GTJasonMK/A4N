//! workflow task projection tests.

use super::task_info_summary;
use a4n_progress::{ProgressMeta, ProgressPhaseKind};
use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority, TaskProgress, TaskStatus};
use chrono::NaiveDate;

#[test]
fn task_history_projection_stays_in_the_workflow_owner() {
    let created_at = NaiveDate::from_ymd_opt(2026, 9, 10)
        .unwrap()
        .and_hms_opt(1, 2, 3)
        .unwrap();
    let started_at = NaiveDate::from_ymd_opt(2026, 9, 10)
        .unwrap()
        .and_hms_opt(1, 2, 4)
        .unwrap();
    let task = TaskInfo {
        id: "task-1".to_string(),
        task_type: TaskKind::MangaGeneration,
        priority: TaskPriority::Medium,
        status: TaskStatus::Running,
        project_id: Some(11),
        root_project_id: Some(7),
        source_snapshot_id: Some(13),
        task_scope: Some("chapter".to_string()),
        progress: Some(TaskProgress {
            current: 2,
            total: 4,
            message: "drawing".to_string(),
            meta: Some(ProgressMeta::new(
                "manga_generation",
                "draw",
                "draw",
                ProgressPhaseKind::AssetGeneration,
            )),
            updated_at: None,
        }),
        created_at,
        started_at: Some(started_at),
        completed_at: None,
    };

    let summary = task_info_summary(&task);

    assert_eq!(summary.id, "task-1");
    assert_eq!(summary.task_type, "manga_generation");
    assert_eq!(summary.priority, "medium");
    assert_eq!(summary.status, "running");
    assert_eq!(summary.root_project_id, Some(7));
    assert_eq!(summary.source_snapshot_id, Some(13));
    assert_eq!(summary.task_scope.as_deref(), Some("chapter"));
    assert_eq!(summary.created_at, created_at.to_string());
    let started_at_text = started_at.to_string();
    assert_eq!(
        summary.started_at.as_deref(),
        Some(started_at_text.as_str())
    );
    let progress = summary.progress.expect("progress should be projected");
    assert_eq!(progress.current, 2);
    assert_eq!(progress.total, 4);
    assert_eq!(progress.message, "drawing");
    assert_eq!(progress.meta.unwrap().stage_code, "draw");
}
