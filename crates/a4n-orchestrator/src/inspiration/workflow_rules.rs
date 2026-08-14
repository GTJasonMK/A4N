//! 灵感对话工作流指纹与 business key 纯规则（无 DB）。

use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

use crate::generation::generation_checkpoint_fingerprint_sha256;
use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 灵感对话工作流输入指纹。
pub fn inspiration_workflow_fingerprint(
    project_id: i64,
    mode: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    if project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    if mode.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "灵感对话 mode 不能为空".to_string(),
        ));
    }
    Ok(json!({
        "workflow": "inspiration_conversation",
        "schema_version": "inspiration_conversation_workflow_v1",
        "project_id": project_id,
        "mode": mode,
        "variables": {},
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

/// 灵感对话 exclusive business key。
pub fn inspiration_business_key(fingerprint: &Value) -> Result<String, AppError> {
    Ok(format!(
        "inspiration_conversation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}

/// 构造灵感对话 begin 输入（status=RUNNING）。
pub fn inspiration_workflow_run_input(
    project_id: i64,
    mode: &str,
    request: &ChatRequest,
    active_task_id: Option<String>,
) -> Result<UpsertWorkflowRunInput, AppError> {
    let fingerprint = inspiration_workflow_fingerprint(project_id, mode, request)?;
    Ok(UpsertWorkflowRunInput {
        business_key: inspiration_business_key(&fingerprint)?,
        workflow_kind: "inspiration_conversation".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("conversation".to_string()),
        scope_id: None,
        chapter_id: None,
        mode: Some(mode.to_string()),
        input_fingerprint_json: Some(fingerprint),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(format!(
            "inspiration_conversation.project_{project_id}.turn"
        )),
        active_task_id,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm::provider::ChatMessage;

    fn sample_request() -> ChatRequest {
        ChatRequest::new(vec![ChatMessage::user("hello")], None, None, None)
    }

    #[test]
    fn rejects_empty_mode() {
        assert!(inspiration_workflow_fingerprint(1, "  ", &sample_request()).is_err());
    }

    #[test]
    fn builds_running_input() {
        let input = inspiration_workflow_run_input(2, "chat", &sample_request(), None).unwrap();
        assert_eq!(input.workflow_kind, "inspiration_conversation");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert!(input.business_key.starts_with("inspiration_conversation:"));
    }
}
