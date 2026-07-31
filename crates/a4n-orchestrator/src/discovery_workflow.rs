use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

pub(crate) fn discovery_business_key(
    workflow_kind: &str,
    project_id: i64,
    chapter_number: Option<i32>,
) -> String {
    match chapter_number {
        Some(chapter_number) => {
            format!("{workflow_kind}:{project_id}:chapter:{chapter_number}")
        }
        None => format!("{workflow_kind}:{project_id}:all"),
    }
}

pub(crate) fn discovery_workflow_fingerprint(
    workflow_kind: &str,
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
) -> Value {
    json!({
        "workflow": workflow_kind,
        "contract_version": 1,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "chapters": chapters.iter().map(|chapter| json!({
            "id": chapter.id,
            "chapter_number": chapter.chapter_number,
            "content_sha256": source_content_sha256(&chapter.content),
            "analysis_data": chapter.analysis_data})).collect::<Vec<_>>(),
        "text_anchors": text_anchors.iter().map(|anchor| json!({
            "id": anchor.id,
            "source_chapter_id": anchor.source_chapter_id,
            "source_id": anchor.source_id,
            "quote_hash": anchor.quote_hash,
            "start_offset": anchor.start_offset,
            "end_offset": anchor.end_offset})).collect::<Vec<_>>(),
        "system_prompt_sha256": request.messages.first()
            .map(|message| source_content_sha256(&message.content)),
        "temperature": request.temperature,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format
    })
}

pub(crate) fn discovery_call_fingerprint_json(
    workflow_kind: &str,
    call_key: &str,
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": workflow_kind,
        "call_key": call_key,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "request": context_aware_chat_request_trace_json(request).map_err(|err| err.to_string())?
    }))
}

pub(crate) fn discovery_workflow_run_input(
    workflow_kind: &str,
    call_key: &str,
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
    active_task_id: Option<&str>,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    let mode = chapter_number
        .map(|value| format!("chapter:{value}"))
        .unwrap_or_else(|| "all".to_string());
    Ok(UpsertWorkflowRunInput {
        business_key: discovery_business_key(workflow_kind, project_id, chapter_number),
        workflow_kind: workflow_kind.to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("novel_project".to_string()),
        scope_id: Some(project_id),
        chapter_id: None,
        mode: Some(mode),
        input_fingerprint_json: Some(discovery_workflow_fingerprint(
            workflow_kind,
            project_id,
            chapter_number,
            chapters,
            text_anchors,
            request,
        )),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(call_key.to_string()),
        active_task_id: active_task_id.map(str::to_string),
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    })
}
