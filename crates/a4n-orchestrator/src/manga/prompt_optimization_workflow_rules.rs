//! 漫画提示词优化工作流 business key 纯规则（无 DB）。

use serde_json::Value;

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 漫画提示词优化 exclusive business key。
pub fn manga_prompt_optimization_business_key(
    project_id: i64,
    prompt_id: i64,
) -> Result<String, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if prompt_id <= 0 {
        return Err("漫画提示词 ID 必须大于 0".to_string());
    }
    Ok(format!(
        "manga_prompt_optimization:{project_id}:{prompt_id}"
    ))
}

/// 构造漫画提示词优化 begin 输入（status=RUNNING）。
pub fn manga_prompt_optimization_workflow_run_input(
    project_id: i64,
    prompt_id: i64,
    chapter_id: i64,
    prompt_type: &str,
    call_key: &str,
    input_fingerprint_json: Value,
) -> Result<UpsertWorkflowRunInput, String> {
    if chapter_id <= 0 {
        return Err("章节 ID 必须大于 0".to_string());
    }
    if call_key.trim().is_empty() {
        return Err("优化 call_key 不能为空".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: manga_prompt_optimization_business_key(project_id, prompt_id)?,
        workflow_kind: "manga_prompt_optimization".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("chapter_manga_prompt".to_string()),
        scope_id: Some(prompt_id),
        chapter_id: Some(chapter_id),
        mode: Some(prompt_type.to_string()),
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
    fn rejects_invalid_ids() {
        assert!(manga_prompt_optimization_business_key(0, 1).is_err());
        assert!(manga_prompt_optimization_business_key(1, 0).is_err());
    }

    #[test]
    fn builds_running_input() {
        let input = manga_prompt_optimization_workflow_run_input(
            2,
            3,
            4,
            "page",
            "optimize_manga_prompt:prompt:3",
            json!({"a": 1}),
        )
        .unwrap();
        assert_eq!(input.business_key, "manga_prompt_optimization:2:3");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
    }
}
