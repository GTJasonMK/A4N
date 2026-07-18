use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use serde_json::{Value, json};

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::context_aware_system_user_request;

use super::super::super::context::part_context_json;
use super::super::super::part_contract::part_generation_manifest_user_message;
use super::super::contracts::PartOutlineRequestInput;
use super::manifest::build_part_outline_context_manifest;

pub(in crate::outline::parts) async fn build_part_outline_request(
    input: PartOutlineRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let PartOutlineRequestInput {
        project_id,
        part_num,
        start_ch,
        end_ch,
        chapters_per_part,
        requested_part_count,
        blueprint_context,
        previous_parts,
        prompt_renderer,
        variables,
    } = input;
    let prompt_content = prompt_renderer
        .render_prompt("part_outline_single", variables, Some(project_id))
        .await?;
    let user_content = part_generation_manifest_user_message(
        chapters_per_part,
        requested_part_count,
        part_num,
        start_ch,
        end_ch,
        previous_parts,
    )?;
    let manifest = build_part_outline_context_manifest(
        blueprint_context,
        previous_parts,
        user_content,
        part_num,
        start_ch,
        end_ch,
    )?;

    Ok(context_aware_system_user_request(
        prompt_content,
        manifest,
        Some(0.7),
        None,
        Some(llm_token_profiles::OUTLINE.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: "part_outline_generation".to_string(),
        step_key: format!("part_{part_num}"),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(format!("part_{part_num}")),
        input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(&json!({
            "blueprint_context": blueprint_context,
            "previous_parts": previous_parts.iter().map(part_context_json).collect::<Vec<_>>(),
            "generation_request": {
                "chapters_per_part": chapters_per_part,
                "requested_part_count": requested_part_count,
                "part_number": part_num,
                "start_chapter": start_ch,
                "end_chapter": end_ch,
            },
        }))?)),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

pub(in crate::outline::parts) fn part_outline_call_fingerprint_json(
    call_key: &str,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
    chapters_per_part: i32,
    requested_part_count: i32,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "part_outline_generation",
        "call_key": call_key,
        "prompt_name": "part_outline_single",
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
        "target": {
            "part_number": part_num,
            "start_chapter": start_ch,
            "end_chapter": end_ch,
            "chapters_per_part": chapters_per_part,
            "requested_part_count": requested_part_count,
        },
    }))
}
