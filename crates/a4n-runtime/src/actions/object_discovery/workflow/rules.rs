use super::super::progress::OBJECT_DISCOVERY_CALL_KEY;
use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

pub(in crate::actions::object_discovery) fn object_discovery_business_key(
    project_id: i64,
    chapter_number: Option<i32>,
) -> String {
    match chapter_number {
        Some(chapter_number) => format!("object_discovery:{project_id}:chapter:{chapter_number}"),
        None => format!("object_discovery:{project_id}:all"),
    }
}

pub(in crate::actions::object_discovery) fn object_discovery_workflow_fingerprint(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
) -> Value {
    json!({
        "workflow": "object_discovery",
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

pub(in crate::actions::object_discovery) fn object_discovery_call_fingerprint_json(
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "object_discovery",
        "call_key": OBJECT_DISCOVERY_CALL_KEY,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "request": context_aware_chat_request_trace_json(request).map_err(|err| err.to_string())?}))
}
