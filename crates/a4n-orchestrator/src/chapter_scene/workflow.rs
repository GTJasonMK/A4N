use a4n_core::error::AppError;
use a4n_llm::{provider::ChatRequest, trace::context_aware_chat_request_trace_json};
use serde_json::{Value, json};

use crate::{
    generation::generation_checkpoint_fingerprint_sha256,
    workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING},
};

pub(super) fn chapter_scene_workflow_fingerprint(
    chapter_id: i64,
    scene_number: i32,
    scene_prompt: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    if chapter_id <= 0 {
        return Err(AppError::InvalidArgument("章节 ID 必须大于 0".to_string()));
    }
    Ok(json!({
        "workflow": "chapter_scene_generation",
        "schema_version": "chapter_scene_generation_workflow_v1",
        "chapter_id": chapter_id,
        "scene_number": scene_number,
        "scene_prompt": scene_prompt.trim(),
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

fn chapter_scene_business_key(fingerprint: &Value) -> Result<String, AppError> {
    Ok(format!(
        "chapter_scene_generation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}

pub(super) fn chapter_scene_call_fingerprint_json(
    call_key: &str,
    workflow_fingerprint: &Value,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "chapter_scene_generation_call_v1",
        "workflow": "chapter_scene_generation",
        "call_key": call_key,
        "workflow_fingerprint": workflow_fingerprint,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

pub(super) fn chapter_scene_workflow_run_input(
    project_id: i64,
    chapter_id: i64,
    scene_number: i32,
    scene_prompt: &str,
    call_key: &str,
    request: &ChatRequest,
    active_task_id: Option<&str>,
) -> Result<(UpsertWorkflowRunInput, Value), AppError> {
    if project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    if call_key.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "场景生成 call_key 不能为空".to_string(),
        ));
    }
    let fingerprint =
        chapter_scene_workflow_fingerprint(chapter_id, scene_number, scene_prompt, request)?;
    let input = UpsertWorkflowRunInput {
        business_key: chapter_scene_business_key(&fingerprint)?,
        workflow_kind: "chapter_scene_generation".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("chapter".to_string()),
        scope_id: Some(chapter_id),
        chapter_id: Some(chapter_id),
        mode: Some("scene".to_string()),
        input_fingerprint_json: Some(fingerprint.clone()),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(call_key.to_string()),
        active_task_id: active_task_id.map(str::to_string),
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    };
    Ok((input, fingerprint))
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm::provider::ChatMessage;

    fn sample_request() -> ChatRequest {
        ChatRequest::new(vec![ChatMessage::user("scene")], None, None, None)
    }

    #[test]
    fn rejects_non_positive_chapter() {
        assert!(chapter_scene_workflow_fingerprint(0, 1, "p", &sample_request()).is_err());
    }

    #[test]
    fn builds_running_input() {
        let (input, _) = chapter_scene_workflow_run_input(
            2,
            3,
            1,
            "prompt",
            "call",
            &sample_request(),
            Some("chapter-scene-task"),
        )
        .expect("valid workflow input");
        assert_eq!(input.workflow_kind, "chapter_scene_generation");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert_eq!(input.active_task_id.as_deref(), Some("chapter-scene-task"));
    }
}
