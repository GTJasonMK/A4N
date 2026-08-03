//! 导入逆向工程工作流指纹与 business key 纯规则（无 DB）。

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// import_reverse exclusive business key。
pub fn import_reverse_business_key(
    project_id: i64,
    prompt_name: &str,
    apply: bool,
) -> Result<String, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if prompt_name.trim().is_empty() {
        return Err("逆向 prompt_name 不能为空".to_string());
    }
    let mode = if apply { "apply" } else { "reverse" };
    Ok(format!(
        "import_reverse_engineer:project:{project_id}:{prompt_name}:{mode}"
    ))
}

/// import_reverse 工作流输入指纹。
pub fn import_reverse_workflow_fingerprint(
    project_id: i64,
    prompt_name: &str,
    apply: bool,
    content_sample: &str,
    auto_context: bool,
    context_limit: Option<i64>,
    variables: &HashMap<String, Value>,
) -> Value {
    let mode = if apply { "apply" } else { "reverse" };
    let variables_json = Value::Object(
        variables
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Map<String, Value>>(),
    );
    json!({
        "schema_version": 1,
        "workflow": "import_reverse_engineer",
        "target": prompt_name,
        "mode": mode,
        "project_id": project_id,
        "content_sample_sha256": source_content_sha256(content_sample),
        "auto_context": auto_context,
        "context_limit": context_limit,
        "variables": variables_json
    })
}

/// 构造 import_reverse begin 输入（status=RUNNING）。
pub fn import_reverse_workflow_run_input(
    project_id: i64,
    prompt_name: &str,
    apply: bool,
    content_sample: &str,
    auto_context: bool,
    context_limit: Option<i64>,
    variables: &HashMap<String, Value>,
    active_task_id: Option<&str>,
) -> Result<UpsertWorkflowRunInput, String> {
    let mode = if apply { "apply" } else { "reverse" };
    Ok(UpsertWorkflowRunInput {
        business_key: import_reverse_business_key(project_id, prompt_name, apply)?,
        workflow_kind: "import_reverse_engineer".to_string(),
        project_id: Some(project_id),
        root_project_id: Some(project_id),
        source_snapshot_id: None,
        scope_table: Some("project".to_string()),
        scope_id: Some(project_id),
        chapter_id: None,
        mode: Some(mode.to_string()),
        input_fingerprint_json: Some(import_reverse_workflow_fingerprint(
            project_id,
            prompt_name,
            apply,
            content_sample,
            auto_context,
            context_limit,
            variables,
        )),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(prompt_name.to_string()),
        active_task_id: active_task_id.map(str::to_string),
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_prompt() {
        assert!(import_reverse_business_key(1, "  ", false).is_err());
    }

    #[test]
    fn builds_running_input() {
        let vars = HashMap::new();
        let input = import_reverse_workflow_run_input(
            2,
            "outline",
            false,
            "sample",
            true,
            Some(3),
            &vars,
            Some("import-reverse-task"),
        )
        .unwrap();
        assert!(input.business_key.contains("outline:reverse"));
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert_eq!(input.active_task_id.as_deref(), Some("import-reverse-task"));
    }
}
