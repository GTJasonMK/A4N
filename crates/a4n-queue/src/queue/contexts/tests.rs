use super::id::required_positive_id;
use super::scope::validate_task_scope;
use super::{TaskFamilyContext, TaskWorkflowContext};

#[test]
fn family_context_preserves_valid_ids_and_scope() {
    let context = TaskFamilyContext::new(11, 7, Some(3), "assembly").unwrap();
    assert_eq!(context.project_id, 11);
    assert_eq!(context.root_project_id, 7);
    assert_eq!(context.source_snapshot_id, Some(3));
    assert_eq!(context.task_scope, "assembly");
}

#[test]
fn workflow_context_preserves_positive_id() {
    assert_eq!(TaskWorkflowContext::new(9).unwrap().workflow_run_id, 9);
}

#[test]
fn rejects_zero_and_negative_ids_with_field_name() {
    for (value, field) in [
        (0, "workflow_run_id"),
        (-1, "project_id"),
        (0, "root_project_id"),
        (-2, "source_snapshot_id"),
    ] {
        let error = required_positive_id(value, field).unwrap_err();
        assert_eq!(error.to_string(), format!("参数无效：{field} 必须大于 0"));
    }
}

#[test]
fn accepts_all_supported_task_scopes() {
    for scope in [
        "text_analysis",
        "source_sync",
        "video_prompt",
        "media_generation",
        "assembly",
    ] {
        assert_eq!(validate_task_scope(scope.to_string()).unwrap(), scope);
    }
}

#[test]
fn rejects_unknown_task_scope() {
    let error = validate_task_scope("unknown_scope".to_string()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "参数无效：task_scope 无效: unknown_scope"
    );
}
