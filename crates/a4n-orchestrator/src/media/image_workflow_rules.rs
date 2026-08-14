//! 图片生成 / 批量图片工作流指纹与 business key 纯规则（无 DB）。

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};

use crate::generation::generation_checkpoint_fingerprint_sha256;
use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 批量图片 exclusive workflow business key。
pub fn batch_image_workflow_business_key(
    project_id: i64,
    requests: &[Value],
) -> Result<String, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    let serialized =
        serde_json::to_string(requests).map_err(|err| format!("批量图片请求序列化失败: {err}"))?;
    Ok(format!(
        "image_batch:{project_id}:{}",
        source_content_sha256(&serialized)
    ))
}

/// 批量图片工作流输入指纹。
pub fn batch_image_workflow_fingerprint(project_id: i64, requests: &[Value]) -> Value {
    json!({
        "workflow": "image_batch",
        "project_id": project_id,
        "requests": requests,
    })
}

/// 批量图片单项 step key。
pub fn batch_image_item_step_key(index: usize) -> String {
    format!("image_batch.item.{}", index + 1)
}

/// 单张图片生成工作流输入指纹。
pub fn image_generation_workflow_fingerprint(
    project_id: i64,
    chapter_id: Option<i64>,
    command: &str,
    image_type: &str,
    panel_id: Option<&str>,
    request_json: &Value,
    metadata_json: Option<&Value>,
) -> Value {
    json!({
        "workflow": "image_generation",
        "project_id": project_id,
        "chapter_id": chapter_id,
        "command": command,
        "image_type": image_type,
        "panel_id": panel_id,
        "request": request_json,
        "metadata": metadata_json,
    })
}

/// 单张图片生成 step key。
pub fn image_generation_step_key(command: &str) -> String {
    format!("image_generation.{command}")
}

/// 单张图片生成 exclusive workflow business key。
pub fn image_generation_workflow_business_key(
    project_id: i64,
    command: &str,
    fingerprint_json: &Value,
) -> Result<String, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if command.trim().is_empty() {
        return Err("图片生成 command 不能为空".to_string());
    }
    let fingerprint_sha = generation_checkpoint_fingerprint_sha256(fingerprint_json)
        .map_err(|err| err.to_string())?;
    Ok(format!(
        "image_generation:{project_id}:{command}:{fingerprint_sha}"
    ))
}

/// 构造批量图片 begin 输入（status=RUNNING）。
pub fn batch_image_workflow_run_input(
    project_id: i64,
    requests: &[Value],
) -> Result<UpsertWorkflowRunInput, String> {
    Ok(UpsertWorkflowRunInput {
        business_key: batch_image_workflow_business_key(project_id, requests)?,
        workflow_kind: "image_batch".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("novel_project".to_string()),
        scope_id: Some(project_id),
        chapter_id: None,
        mode: Some("batch".to_string()),
        input_fingerprint_json: Some(batch_image_workflow_fingerprint(project_id, requests)),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some("image_batch.prepare".to_string()),
        active_task_id: None,
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    })
}

/// 构造单张图片生成 begin 输入（status=RUNNING）。
pub fn image_generation_workflow_run_input(
    project_id: i64,
    chapter_id: Option<i64>,
    command: &str,
    image_type: &str,
    panel_id: Option<&str>,
    request_json: &Value,
    metadata_json: Option<&Value>,
) -> Result<(UpsertWorkflowRunInput, String, Value), String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if command.trim().is_empty() {
        return Err("图片生成 command 不能为空".to_string());
    }
    if image_type.trim().is_empty() {
        return Err("图片类型不能为空".to_string());
    }
    let fingerprint_json = image_generation_workflow_fingerprint(
        project_id,
        chapter_id,
        command,
        image_type,
        panel_id,
        request_json,
        metadata_json,
    );
    let step_key = image_generation_step_key(command);
    let input = UpsertWorkflowRunInput {
        business_key: image_generation_workflow_business_key(
            project_id,
            command,
            &fingerprint_json,
        )?,
        workflow_kind: "image_generation".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("novel_project".to_string()),
        scope_id: Some(project_id),
        chapter_id,
        mode: Some(image_type.to_string()),
        input_fingerprint_json: Some(fingerprint_json.clone()),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(step_key.clone()),
        active_task_id: None,
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    };
    Ok((input, step_key, fingerprint_json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_business_key_stable_for_same_requests() {
        let requests = vec![json!({"a": 1})];
        let key = batch_image_workflow_business_key(7, &requests).expect("key");
        assert!(key.starts_with("image_batch:7:"));
        assert_eq!(
            key,
            batch_image_workflow_business_key(7, &requests).unwrap()
        );
    }

    #[test]
    fn image_generation_run_input_is_running() {
        let (input, step_key, fp) = image_generation_workflow_run_input(
            3,
            Some(9),
            "generate",
            "scene",
            None,
            &json!({"p": 1}),
            None,
        )
        .expect("input");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert_eq!(step_key, "image_generation.generate");
        assert_eq!(fp["project_id"], 3);
        assert!(
            input
                .business_key
                .starts_with("image_generation:3:generate:")
        );
    }

    #[test]
    fn rejects_non_positive_project() {
        assert!(batch_image_workflow_business_key(0, &[]).is_err());
        assert!(
            image_generation_workflow_run_input(0, None, "c", "t", None, &json!({}), None).is_err()
        );
    }
}
