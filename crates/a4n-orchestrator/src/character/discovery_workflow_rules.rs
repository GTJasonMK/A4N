//! 角色发现工作流指纹与 business key 纯规则（无 DB）。

use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 角色发现 LLM 调用键。
pub const CHARACTER_DISCOVERY_CALL_KEY: &str = "character_discovery.llm";

/// 角色发现 exclusive workflow business key。
pub fn character_discovery_business_key(project_id: i64, chapter_number: Option<i32>) -> String {
    match chapter_number {
        Some(chapter_number) => {
            format!("character_discovery:{project_id}:chapter:{chapter_number}")
        }
        None => format!("character_discovery:{project_id}:all"),
    }
}

/// 角色发现工作流输入指纹。
pub fn character_discovery_workflow_fingerprint(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
) -> Value {
    json!({
        "workflow": "character_discovery",
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
        "response_format": request.response_format})
}

/// 角色发现单次 LLM 调用指纹。
pub fn character_discovery_call_fingerprint_json(
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "character_discovery",
        "call_key": CHARACTER_DISCOVERY_CALL_KEY,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "request": context_aware_chat_request_trace_json(request).map_err(|err| err.to_string())?}))
}

/// 构造角色发现 begin 输入（status=RUNNING）。
pub fn character_discovery_workflow_run_input(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    let mode = chapter_number
        .map(|value| format!("chapter:{value}"))
        .unwrap_or_else(|| "all".to_string());
    Ok(UpsertWorkflowRunInput {
        business_key: character_discovery_business_key(project_id, chapter_number),
        workflow_kind: "character_discovery".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("novel_project".to_string()),
        scope_id: Some(project_id),
        chapter_id: None,
        mode: Some(mode),
        input_fingerprint_json: Some(character_discovery_workflow_fingerprint(
            project_id,
            chapter_number,
            chapters,
            text_anchors,
            request,
        )),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(CHARACTER_DISCOVERY_CALL_KEY.to_string()),
        active_task_id: None,
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm::provider::ChatMessage;

    #[test]
    fn business_key_distinguishes_chapter_and_all() {
        assert_eq!(
            character_discovery_business_key(3, Some(2)),
            "character_discovery:3:chapter:2"
        );
        assert_eq!(
            character_discovery_business_key(3, None),
            "character_discovery:3:all"
        );
    }

    #[test]
    fn workflow_run_input_rejects_non_positive_project() {
        let request = ChatRequest::new(vec![ChatMessage::user("hi")], None, None, None);
        assert!(character_discovery_workflow_run_input(0, None, &[], &[], &request).is_err());
    }
}
