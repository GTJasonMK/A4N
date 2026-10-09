use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use serde_json::json;

use crate::outline::chapter_contract::chapter_outline_generation_manifest_user_message;
use crate::outline::context::part_context_json;
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm_context::context_aware_system_user_request;

use super::super::contracts::{ChapterOutlineRequestInput, ContextAwareChapterOutlineRequestInput};
use super::context::build_chapter_outline_context_manifest;

pub(in crate::outline::chapters) async fn build_chapter_outline_request(
    input: ChapterOutlineRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let ChapterOutlineRequestInput {
        project_id,
        prompt_assembly_call_key,
        context,
        focus_part,
        start_chapter,
        count,
        end_chapter,
        prompt_renderer,
        variables,
    } = input;
    let prompt_content = prompt_renderer
        .render_prompt_for_call(
            prompt_assembly_call_key,
            "outline",
            variables,
            Some(project_id),
        )
        .await?;
    let user_content = chapter_outline_generation_manifest_user_message(
        focus_part,
        start_chapter,
        count,
        end_chapter,
    )?;
    Ok(
        build_context_aware_chapter_outline_request(ContextAwareChapterOutlineRequestInput {
            project_id,
            context,
            prompt_content,
            user_content,
            count,
            start_chapter,
            end_chapter,
            temperature: Some(0.7),
            workflow: "chapter_outline_generation",
            step_key: format!("chapter_{start_chapter}"),
        })?
        .with_prompt_assembly_key(prompt_assembly_call_key),
    )
}

pub(in crate::outline::chapters) fn build_context_aware_chapter_outline_request(
    input: ContextAwareChapterOutlineRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let ContextAwareChapterOutlineRequestInput {
        project_id,
        context,
        prompt_content,
        user_content,
        count,
        start_chapter,
        end_chapter,
        temperature,
        workflow,
        step_key,
    } = input;
    let dynamic_user_content_sha256 = source_content_sha256(&user_content);
    let manifest = build_chapter_outline_context_manifest(context, user_content)?;
    Ok(context_aware_system_user_request(
        prompt_content,
        manifest,
        temperature,
        None,
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: workflow.to_string(),
        step_key: step_key.clone(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(step_key),
        input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(&json!({
            "blueprint_context": context.blueprint_payload,
            "part_outlines": context.parts.iter().map(part_context_json).collect::<Vec<_>>(),
            "source_snapshot_sha256": source_content_sha256(&serde_json::to_string(&context.source_snapshot)?),
            "range": {
                "start_chapter": start_chapter,
                "count": count,
                "end_chapter": end_chapter,
            },
            "dynamic_user_content_sha256": dynamic_user_content_sha256,
        }))?)),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}
