use chrono::{Duration, NaiveDate};

use super::WorkflowRunModelBuilder;

fn timestamp() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("valid date")
        .and_hms_opt(3, 4, 5)
        .expect("valid time")
}

#[test]
fn workflow_run_builder_has_neutral_optional_defaults() {
    let timestamp = timestamp();
    let model = WorkflowRunModelBuilder::new(7, "business", "test", "running", timestamp).build();

    assert_eq!(model.id, 7);
    assert_eq!(model.business_key, "business");
    assert_eq!(model.workflow_kind, "test");
    assert_eq!(model.status, "running");
    assert_eq!(model.created_at, timestamp);
    assert_eq!(model.updated_at, timestamp);
    assert_eq!(model.project_id, None);
    assert_eq!(model.scope_table, None);
    assert_eq!(model.current_step_key, None);
    assert!(!model.can_resume);
    assert_eq!(model.completed_at, None);
}

#[test]
fn workflow_run_builder_projects_scope_state_and_timestamps() {
    let timestamp = timestamp();
    let updated_at = timestamp + Duration::minutes(1);
    let model = WorkflowRunModelBuilder::new(9, "chapter:2", "chapter", "failed", timestamp)
        .with_project_scope(Some(3), Some(1))
        .with_source_snapshot_id(4)
        .with_scope("chapter", 2)
        .with_chapter_id(2)
        .with_mode("resume")
        .with_input_fingerprint(serde_json::json!({"chapter_id": 2}))
        .with_current_step("analysis")
        .with_progress(serde_json::json!({"current": 1, "total": 2}))
        .with_active_task("task-9")
        .with_resumability(true, Some("retry".to_string()))
        .with_error_message("provider failed")
        .with_updated_at(updated_at)
        .with_completed_at(Some(updated_at))
        .build();

    assert_eq!(model.project_id, Some(3));
    assert_eq!(model.root_project_id, Some(1));
    assert_eq!(model.source_snapshot_id, Some(4));
    assert_eq!(model.scope_table.as_deref(), Some("chapter"));
    assert_eq!(model.scope_id, Some(2));
    assert_eq!(model.chapter_id, Some(2));
    assert_eq!(model.mode.as_deref(), Some("resume"));
    assert_eq!(model.current_step_key.as_deref(), Some("analysis"));
    assert_eq!(model.active_task_id.as_deref(), Some("task-9"));
    assert!(model.can_resume);
    assert_eq!(model.resume_block_reason.as_deref(), Some("retry"));
    assert_eq!(model.error_message.as_deref(), Some("provider failed"));
    assert_eq!(model.updated_at, updated_at);
    assert_eq!(model.completed_at, Some(updated_at));
}
