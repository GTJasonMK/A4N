use a4n_api_contract::workflow::{
    CancelWorkflowInput, GetWorkflowProgressEventsInput, WorkflowProgressEventsInput,
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
    let task_input = serde_json::from_value::<GetWorkflowProgressEventsInput>(json!({
        "task_id": "task-1",
        "limit": 20,
    }))
    .unwrap();
    assert_eq!(task_input.task_id.as_deref(), Some("task-1"));
    assert_eq!(task_input.limit, 20);
    assert_eq!(task_input.after_event_id, None);
    let cursor_input = serde_json::from_value::<GetWorkflowProgressEventsInput>(json!({
        "task_id": "task-1",
        "after_event_id": "task-1-2026-10-15T10:00:00Z",
        "limit": 20,
    }))
    .unwrap();
    assert_eq!(
        cursor_input.after_event_id.as_deref(),
        Some("task-1-2026-10-15T10:00:00Z")
    );
    let workflow_input = serde_json::from_value::<GetWorkflowProgressEventsInput>(json!({
        "workflow_run_id": 9,
        "limit": 20,
    }))
    .unwrap();
    assert_eq!(workflow_input.workflow_run_id, Some(9));
    let persisted_input = WorkflowProgressEventsInput {
        task_id: None,
        workflow_run_id: Some(9),
        after_event_id: Some(17),
        limit: 20,
    };
    assert_eq!(persisted_input.after_event_id, Some(17));
    assert!(
        serde_json::from_value::<GetWorkflowProgressEventsInput>(json!({"workflowRunId": 9}))
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );
}
