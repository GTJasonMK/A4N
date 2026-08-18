//! 导入逆向工程工作流指纹与 business key 纯规则（无 DB）。

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

use super::ReverseEngineeringTarget;
use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

pub(crate) const IMPORT_REVERSE_WORKFLOW_KIND: &str = "import_reverse_engineer";
pub(crate) const IMPORT_REVERSE_WORKFLOW_MODE: &str = "reverse";

/// import_reverse exclusive business key。
pub fn import_reverse_business_key(
    project_id: i64,
    target: ReverseEngineeringTarget,
) -> Result<String, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    let prompt_name = target.prompt_name();
    Ok(format!(
        "{IMPORT_REVERSE_WORKFLOW_KIND}:project:{project_id}:{prompt_name}:{IMPORT_REVERSE_WORKFLOW_MODE}"
    ))
}

/// import_reverse 工作流输入指纹。
pub fn import_reverse_workflow_fingerprint(
    project_id: i64,
    target: ReverseEngineeringTarget,
    content_sample: &str,
    auto_context: bool,
    context_limit: Option<i64>,
    auto_context_source_sha256: Option<&str>,
    variables: &HashMap<String, Value>,
) -> Value {
    let prompt_name = target.prompt_name();
    let variables_json = Value::Object(
        variables
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Map<String, Value>>(),
    );
    json!({
        "schema_version": 1,
        "workflow": IMPORT_REVERSE_WORKFLOW_KIND,
        "target": prompt_name,
        "mode": IMPORT_REVERSE_WORKFLOW_MODE,
        "project_id": project_id,
        "content_sample_sha256": source_content_sha256(content_sample),
        "auto_context": auto_context,
        "context_limit": context_limit,
        "auto_context_source_sha256": auto_context_source_sha256,
        "variables": variables_json
    })
}

/// 构造 import_reverse begin 输入（status=RUNNING）。
pub fn import_reverse_workflow_run_input(
    project_id: i64,
    target: ReverseEngineeringTarget,
    content_sample: &str,
    auto_context: bool,
    context_limit: Option<i64>,
    auto_context_source_sha256: Option<&str>,
    variables: &HashMap<String, Value>,
    active_task_id: Option<&str>,
) -> Result<UpsertWorkflowRunInput, String> {
    let prompt_name = target.prompt_name();
    let valid_source_fingerprint = auto_context_source_sha256.is_some_and(|value| {
        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    });
    if (auto_context && !valid_source_fingerprint)
        || (!auto_context && auto_context_source_sha256.is_some())
    {
        return Err(if auto_context {
            "启用项目上下文时缺少有效的上下文来源指纹".to_string()
        } else {
            "未启用项目上下文时不应提供上下文来源指纹".to_string()
        });
    }
    Ok(UpsertWorkflowRunInput {
        business_key: import_reverse_business_key(project_id, target)?,
        workflow_kind: IMPORT_REVERSE_WORKFLOW_KIND.to_string(),
        project_id: Some(project_id),
        root_project_id: Some(project_id),
        source_snapshot_id: None,
        scope_table: Some("project".to_string()),
        scope_id: Some(project_id),
        chapter_id: None,
        mode: Some(IMPORT_REVERSE_WORKFLOW_MODE.to_string()),
        input_fingerprint_json: Some(import_reverse_workflow_fingerprint(
            project_id,
            target,
            content_sample,
            auto_context,
            context_limit,
            auto_context_source_sha256,
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
    fn rejects_non_positive_project_id() {
        assert!(import_reverse_business_key(0, ReverseEngineeringTarget::Outline).is_err());
    }

    #[test]
    fn builds_running_input() {
        let vars = HashMap::new();
        let source_fingerprint = "a".repeat(64);
        let input = import_reverse_workflow_run_input(
            2,
            ReverseEngineeringTarget::Outline,
            "sample",
            true,
            Some(3),
            Some(&source_fingerprint),
            &vars,
            Some("import-reverse-task"),
        )
        .unwrap();
        assert!(input.business_key.contains("outline:reverse"));
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert_eq!(input.active_task_id.as_deref(), Some("import-reverse-task"));
    }

    #[test]
    fn auto_context_requires_a_source_fingerprint() {
        let vars = HashMap::new();
        assert!(
            import_reverse_workflow_run_input(
                2,
                ReverseEngineeringTarget::Outline,
                "sample",
                true,
                Some(3),
                None,
                &vars,
                None,
            )
            .is_err()
        );
    }
}
