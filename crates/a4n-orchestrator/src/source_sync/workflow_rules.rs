//! 源文本同步 apply 工作流指纹与 business key 纯规则（无 DB）。

use a4n_domain::source_sync::SourceSyncRunMode;
use serde_json::{Value, json};

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// source_sync apply exclusive business key。
pub fn source_sync_apply_business_key(sync_run_id: i64) -> Result<String, String> {
    if sync_run_id <= 0 {
        return Err("sync_run_id 必须大于 0".to_string());
    }
    Ok(format!("source_sync_apply:run:{sync_run_id}"))
}

/// source_sync apply 工作流输入指纹。
pub fn source_sync_apply_workflow_fingerprint(
    sync_run_id: i64,
    derived_project_id: i64,
    root_project_id: i64,
    from_snapshot_id: i64,
    to_snapshot_id: i64,
    decisions: &Value,
) -> Value {
    json!({
        "schema_version": 1,
        "workflow": "source_sync_apply",
        "sync_run_id": sync_run_id,
        "derived_project_id": derived_project_id,
        "root_project_id": root_project_id,
        "from_snapshot_id": from_snapshot_id,
        "to_snapshot_id": to_snapshot_id,
        "decisions": decisions,
    })
}

/// 构造 source_sync apply begin 输入（status=RUNNING）。
pub fn source_sync_apply_workflow_run_input(
    sync_run_id: i64,
    derived_project_id: i64,
    root_project_id: i64,
    from_snapshot_id: i64,
    to_snapshot_id: i64,
    decisions: &Value,
) -> Result<UpsertWorkflowRunInput, String> {
    if derived_project_id <= 0 {
        return Err("derived_project_id 必须大于 0".to_string());
    }
    if root_project_id <= 0 {
        return Err("root_project_id 必须大于 0".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: source_sync_apply_business_key(sync_run_id)?,
        workflow_kind: "source_sync_apply".to_string(),
        project_id: Some(derived_project_id),
        root_project_id: Some(root_project_id),
        source_snapshot_id: Some(to_snapshot_id),
        scope_table: Some("source_sync_run".to_string()),
        scope_id: Some(sync_run_id),
        chapter_id: None,
        mode: Some(SourceSyncRunMode::Apply.as_str().to_string()),
        input_fingerprint_json: Some(source_sync_apply_workflow_fingerprint(
            sync_run_id,
            derived_project_id,
            root_project_id,
            from_snapshot_id,
            to_snapshot_id,
            decisions,
        )),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some("apply_decisions".to_string()),
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
    fn rejects_invalid_sync_run() {
        assert!(source_sync_apply_business_key(0).is_err());
    }

    #[test]
    fn builds_running_input() {
        let input = source_sync_apply_workflow_run_input(1, 2, 3, 4, 5, &json!([])).unwrap();
        assert_eq!(input.business_key, "source_sync_apply:run:1");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
    }
}
