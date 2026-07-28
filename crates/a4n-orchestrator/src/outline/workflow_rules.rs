//! 大纲工作流 business key / begin 输入纯规则（无 DB）。

use serde_json::Value;

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 大纲 exclusive workflow business key。
pub fn outline_workflow_business_key(workflow_kind: &str, project_id: i64, mode: &str) -> String {
    format!("{workflow_kind}:project:{project_id}:{mode}")
}

/// 校验并构造大纲 begin 输入。
pub fn outline_workflow_run_input(
    project_id: i64,
    workflow_kind: &str,
    mode: &str,
    current_step_key: &str,
    task_id: Option<String>,
    input_fingerprint_json: Value,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if workflow_kind.trim().is_empty() {
        return Err("workflow_kind 不能为空".to_string());
    }
    if mode.trim().is_empty() {
        return Err("大纲 workflow mode 不能为空".to_string());
    }
    if current_step_key.trim().is_empty() {
        return Err("current_step_key 不能为空".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: outline_workflow_business_key(workflow_kind, project_id, mode),
        workflow_kind: workflow_kind.to_string(),
        project_id: Some(project_id),
        root_project_id: Some(project_id),
        source_snapshot_id: None,
        scope_table: Some("project".to_string()),
        scope_id: Some(project_id),
        chapter_id: None,
        mode: Some(mode.to_string()),
        input_fingerprint_json: Some(input_fingerprint_json),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(current_step_key.to_string()),
        active_task_id: task_id,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn business_key_format() {
        assert_eq!(
            outline_workflow_business_key("part_outline", 3, "generate"),
            "part_outline:project:3:generate"
        );
    }

    #[test]
    fn begin_input_requires_positive_project() {
        assert!(outline_workflow_run_input(0, "k", "m", "s", None, json!({})).is_err());
        let input =
            outline_workflow_run_input(2, "k", "m", "s", Some("t1".into()), json!({"a": 1}))
                .expect("ok");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert_eq!(input.active_task_id.as_deref(), Some("t1"));
    }
}
