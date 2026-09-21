//! Galgame 管线 workflow begin 纯规则（无 DB）。

use serde_json::{Value, json};

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// Galgame 管线 workflow kind。
pub(super) const GALGAME_PIPELINE_WORKFLOW_KIND: &str = "galgame_pipeline";
/// Galgame 管线 mode。
const GALGAME_PIPELINE_MODE_FULL: &str = "full_pipeline_v1";

/// exclusive business key。
fn galgame_pipeline_business_key(project_id: i64) -> String {
    format!("galgame_pipeline:{project_id}")
}

/// 输入指纹。
fn galgame_pipeline_workflow_fingerprint(
    project_id: i64,
    source_snapshot_id: i64,
    root_project_id: i64,
) -> Value {
    json!({
        "project_id": project_id,
        "source_snapshot_id": source_snapshot_id,
        "root_project_id": root_project_id,
        "mode": GALGAME_PIPELINE_MODE_FULL,
    })
}

/// 构造 begin 输入。
pub(super) fn galgame_pipeline_workflow_run_input(
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
    current_step_key: &str,
    task_id: Option<String>,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if root_project_id <= 0 {
        return Err("根项目 ID 必须大于 0".to_string());
    }
    if source_snapshot_id <= 0 {
        return Err("源快照 ID 必须大于 0".to_string());
    }
    if current_step_key.trim().is_empty() {
        return Err("current_step_key 不能为空".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: galgame_pipeline_business_key(project_id),
        workflow_kind: GALGAME_PIPELINE_WORKFLOW_KIND.to_string(),
        project_id: Some(project_id),
        root_project_id: Some(root_project_id),
        source_snapshot_id: Some(source_snapshot_id),
        scope_table: Some("galgame_script".to_string()),
        scope_id: None,
        chapter_id: None,
        mode: Some(GALGAME_PIPELINE_MODE_FULL.to_string()),
        input_fingerprint_json: Some(galgame_pipeline_workflow_fingerprint(
            project_id,
            source_snapshot_id,
            root_project_id,
        )),
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

    #[test]
    fn business_key_and_begin_gate() {
        assert_eq!(galgame_pipeline_business_key(9), "galgame_pipeline:9");
        assert!(galgame_pipeline_workflow_run_input(0, 1, 1, "s", None).is_err());
        let input =
            galgame_pipeline_workflow_run_input(2, 3, 4, "stage", Some("t".into())).expect("ok");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert_eq!(input.workflow_kind, GALGAME_PIPELINE_WORKFLOW_KIND);
    }
}
