//! 角色隐性属性复审工作流 business key 与 begin 输入纯规则（无 DB）。

use serde_json::Value;

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 隐性属性复审 exclusive business key。
pub(super) fn implicit_update_review_business_key(
    profile_id: i64,
    attribute_key: &str,
) -> Result<String, String> {
    if profile_id <= 0 {
        return Err("角色档案 ID 必须大于 0".to_string());
    }
    let attribute_key = attribute_key.trim();
    if attribute_key.is_empty() {
        return Err("隐性属性 key 不能为空".to_string());
    }
    Ok(format!(
        "implicit_update_review:{profile_id}:{attribute_key}"
    ))
}

/// 构造隐性属性复审 begin 输入（status=RUNNING）。
pub(super) fn implicit_update_review_workflow_run_input(
    project_id: i64,
    profile_id: i64,
    attribute_key: &str,
    call_key: &str,
    input_fingerprint_json: Value,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if call_key.trim().is_empty() {
        return Err("隐性复审 call_key 不能为空".to_string());
    }
    let attribute_key = attribute_key.trim();
    Ok(UpsertWorkflowRunInput {
        business_key: implicit_update_review_business_key(profile_id, attribute_key)?,
        workflow_kind: "implicit_update_review".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("protagonist_profile".to_string()),
        scope_id: Some(profile_id),
        chapter_id: None,
        mode: Some(attribute_key.to_string()),
        input_fingerprint_json: Some(input_fingerprint_json),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(call_key.to_string()),
        active_task_id: None,
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
    fn rejects_empty_attribute_key() {
        assert!(implicit_update_review_business_key(1, "  ").is_err());
    }

    #[test]
    fn builds_running_input() {
        let input = implicit_update_review_workflow_run_input(
            2,
            3,
            "mood",
            "implicit_update_review:3:mood",
            json!({"a": 1}),
        )
        .unwrap();
        assert_eq!(input.business_key, "implicit_update_review:3:mood");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
    }
}
