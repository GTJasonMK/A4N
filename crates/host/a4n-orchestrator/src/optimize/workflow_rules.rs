//! 内容优化会话工作流指纹与 business key 纯规则（无 DB）。

use serde_json::{Value, json};

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 优化会话 exclusive business key。
pub fn optimize_session_workflow_business_key(
    workflow_kind: &str,
    session_id: i64,
    mode: &str,
) -> Result<String, String> {
    if workflow_kind.trim().is_empty() {
        return Err("优化 workflow_kind 不能为空".to_string());
    }
    if session_id <= 0 {
        return Err("优化会话 ID 必须大于 0".to_string());
    }
    if mode.trim().is_empty() {
        return Err("优化 mode 不能为空".to_string());
    }
    Ok(format!("{workflow_kind}:{session_id}:{mode}"))
}

/// 优化会话工作流输入指纹。
pub fn optimize_session_workflow_fingerprint(
    workflow_kind: &str,
    mode: &str,
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Value {
    json!({
        "workflow": workflow_kind,
        "mode": mode,
        "session_id": session_id,
        "project_id": project_id,
        "chapter_id": chapter_id,
    })
}

/// 构造优化会话 begin 输入（status=RUNNING）。
pub fn optimize_session_workflow_run_input(
    session_id: i64,
    workflow_kind: &str,
    mode: &str,
    project_id: i64,
    chapter_id: i64,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if chapter_id <= 0 {
        return Err("章节 ID 必须大于 0".to_string());
    }
    let business_key = optimize_session_workflow_business_key(workflow_kind, session_id, mode)?;
    let current_step_key = format!("optimization_session:{session_id}:{mode}");
    Ok(UpsertWorkflowRunInput {
        business_key,
        workflow_kind: workflow_kind.to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("optimization_session".to_string()),
        scope_id: Some(session_id),
        chapter_id: Some(chapter_id),
        mode: Some(mode.to_string()),
        input_fingerprint_json: Some(optimize_session_workflow_fingerprint(
            workflow_kind,
            mode,
            session_id,
            project_id,
            chapter_id,
        )),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(current_step_key),
        active_task_id: None,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_kind_or_mode() {
        assert!(optimize_session_workflow_business_key("", 1, "m").is_err());
        assert!(optimize_session_workflow_business_key("k", 0, "m").is_err());
        assert!(optimize_session_workflow_business_key("k", 1, "").is_err());
    }

    #[test]
    fn builds_running_input() {
        let input =
            optimize_session_workflow_run_input(9, "content_optimization", "chat", 2, 3).unwrap();
        assert_eq!(input.business_key, "content_optimization:9:chat");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
    }
}
