use super::{
    CancelWorkflowInput, WorkflowProgressEventsInput,
    projections::{validate_optional_project_id, validate_task_history_limit},
};
use serde_json::json;

#[test]
fn cancel_workflow_input_rejects_camel_case_task_id_alias() {
    let input = serde_json::from_value::<CancelWorkflowInput>(json!({
        "task_id": "workflow-1",
    }))
    .unwrap();
    assert_eq!(input.task_id.as_deref(), Some("workflow-1"));
    assert_eq!(input.workflow_run_id, None);
    let run_input = serde_json::from_value::<CancelWorkflowInput>(json!({
        "workflow_run_id": 70,
    }))
    .unwrap();
    assert_eq!(run_input.workflow_run_id, Some(70));
    assert!(
        serde_json::from_value::<CancelWorkflowInput>(json!({"taskId": "workflow-1"}))
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );
}

#[test]
fn workflow_progress_events_input_accepts_task_or_workflow_run_id() {
    let task_input = serde_json::from_value::<WorkflowProgressEventsInput>(json!({
        "task_id": "task-1",
        "limit": 20,
    }))
    .unwrap();
    assert_eq!(task_input.task_id.as_deref(), Some("task-1"));
    assert_eq!(task_input.limit, Some(20));
    let workflow_input = serde_json::from_value::<WorkflowProgressEventsInput>(json!({
        "workflow_run_id": 9,
    }))
    .unwrap();
    assert_eq!(workflow_input.workflow_run_id, Some(9));
    assert!(
        serde_json::from_value::<WorkflowProgressEventsInput>(json!({"workflowRunId": 9}))
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );
}

#[test]
fn task_history_limit_must_be_positive() {
    assert!(validate_task_history_limit(1).is_ok());
    assert_eq!(
        validate_task_history_limit(0).unwrap_err(),
        "limit 必须大于 0"
    );
}

#[test]
fn optional_project_id_must_be_positive_when_present() {
    assert!(validate_optional_project_id(None).is_ok());
    assert!(validate_optional_project_id(Some(1)).is_ok());
    assert_eq!(
        validate_optional_project_id(Some(0)).unwrap_err(),
        "project_id 必须大于 0"
    );
    assert_eq!(
        validate_optional_project_id(Some(-1)).unwrap_err(),
        "project_id 必须大于 0"
    );
}
