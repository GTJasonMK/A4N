use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::{Value, json};

use a4n_prompt::rendering::PromptRenderer;

use super::super::super::chapter_contract::beat_mapping_manifest_user_message;
use super::super::contracts::ContextAwareChapterOutlineRequestInput;
use super::super::requests::build_context_aware_chapter_outline_request;
use super::preparation::PreparedBeatMapping;

pub(super) async fn build_beat_mapping_request(
    project_id: i64,
    prepared: &PreparedBeatMapping,
    prompt_renderer: &dyn PromptRenderer,
    variables: &HashMap<String, Value>,
) -> Result<ChatRequest, AppError> {
    let prompt_content = prompt_renderer
        .render_prompt_for_call(
            "OutlineService.apply_beat_mapping",
            "outline",
            variables,
            Some(project_id),
        )
        .await?;
    let user_content =
        beat_mapping_manifest_user_message(&prepared.part, &prepared.beat_payload, prepared.count)?;
    Ok(
        build_context_aware_chapter_outline_request(ContextAwareChapterOutlineRequestInput {
            project_id,
            context: &prepared.context,
            prompt_content,
            user_content,
            count: prepared.count,
            start_chapter: prepared.part.start_chapter,
            end_chapter: prepared.part.end_chapter,
            temperature: Some(0.6),
            workflow: "chapter_outline_beat_mapping",
            step_key: format!("part_{}_beat_mapping", prepared.part.part_number),
        })?
        .with_prompt_assembly_key("OutlineService.apply_beat_mapping"),
    )
}

pub(super) fn beat_mapping_call_fingerprint_json(
    call_key: &str,
    prepared: &PreparedBeatMapping,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "chapter_outline_beat_mapping",
        "call_key": call_key,
        "prompt_name": "outline",
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "response_format": request.response_format,
        "source_snapshot_sha256": a4n_domain::source_text_hash(
            &serde_json::to_string(&prepared.context.source_snapshot)?
        ),
        "target": {
            "part_id": prepared.part.id,
            "part_number": prepared.part.part_number,
            "start_chapter": prepared.part.start_chapter,
            "end_chapter": prepared.part.end_chapter,
            "count": prepared.count,
        },
        "beat_payload": prepared.beat_payload,
    }))
}
