//! 分镜脚本生成工作流指纹与 business key 纯规则（无 DB）。

use serde_json::{Value, json};

use super::contracts::CHAPTER_SHOOTING_SCRIPT_WORKFLOW;
use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 章节分镜脚本 exclusive business key。
fn chapter_shooting_script_business_key(
    project_id: i64,
    source_chapter_id: i64,
    target_revision: i32,
    visual_plan_version: i32,
) -> Result<String, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if source_chapter_id <= 0 {
        return Err("源章节 ID 必须大于 0".to_string());
    }
    if target_revision <= 0 {
        return Err("分镜脚本 revision 必须大于 0".to_string());
    }
    if visual_plan_version <= 0 {
        return Err("视觉编排版本必须大于 0".to_string());
    }
    Ok(format!(
        "{CHAPTER_SHOOTING_SCRIPT_WORKFLOW}:{project_id}:{source_chapter_id}:{target_revision}:plan:{visual_plan_version}"
    ))
}

/// 章节分镜脚本工作流输入指纹。
fn chapter_shooting_script_workflow_fingerprint(
    project_id: i64,
    source_chapter_id: i64,
    visual_plan_id: i64,
    visual_plan_version: i32,
    visual_plan_sha256: &str,
) -> Value {
    json!({
        "project_id": project_id,
        "source_chapter_id": source_chapter_id,
        "visual_plan_id": visual_plan_id,
        "visual_plan_version": visual_plan_version,
        "visual_plan_sha256": visual_plan_sha256,
    })
}

/// 构造章节分镜脚本 begin 输入（status=RUNNING）。
pub(crate) fn chapter_shooting_script_workflow_run_input(
    project_id: i64,
    source_chapter_id: i64,
    target_revision: i32,
    visual_plan_id: i64,
    visual_plan_version: i32,
    visual_plan_sha256: &str,
    active_task_id: Option<&str>,
) -> Result<UpsertWorkflowRunInput, String> {
    if visual_plan_id <= 0 {
        return Err("视觉编排 ID 必须大于 0".to_string());
    }
    if visual_plan_sha256.len() != 64
        || !visual_plan_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("视觉编排 SHA-256 无效".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: chapter_shooting_script_business_key(
            project_id,
            source_chapter_id,
            target_revision,
            visual_plan_version,
        )?,
        workflow_kind: CHAPTER_SHOOTING_SCRIPT_WORKFLOW.to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some(CHAPTER_SHOOTING_SCRIPT_WORKFLOW.to_string()),
        scope_id: Some(source_chapter_id),
        chapter_id: Some(source_chapter_id),
        mode: None,
        input_fingerprint_json: Some(chapter_shooting_script_workflow_fingerprint(
            project_id,
            source_chapter_id,
            visual_plan_id,
            visual_plan_version,
            visual_plan_sha256,
        )),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some("screenplay_layout".to_string()),
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
    fn rejects_invalid_ids() {
        assert!(chapter_shooting_script_business_key(0, 1, 1, 1).is_err());
        assert!(chapter_shooting_script_business_key(1, 0, 1, 1).is_err());
        assert!(chapter_shooting_script_business_key(1, 1, 0, 1).is_err());
        assert!(chapter_shooting_script_business_key(1, 1, 1, 0).is_err());
    }

    #[test]
    fn builds_running_input() {
        let input = chapter_shooting_script_workflow_run_input(
            2,
            3,
            4,
            9,
            2,
            &"a".repeat(64),
            Some("shooting-task"),
        )
        .unwrap();
        assert_eq!(input.business_key, "chapter_shooting_script:2:3:4:plan:2");
        assert_eq!(input.workflow_kind, CHAPTER_SHOOTING_SCRIPT_WORKFLOW);
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert_eq!(input.active_task_id.as_deref(), Some("shooting-task"));
    }
}
