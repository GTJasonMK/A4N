use std::collections::HashMap;

use a4n_domain::chapter::ChapterRecord;
use a4n_domain::source_spans::source_spans_for_chapters;
use a4n_error::AppError;
use a4n_llm_context::{LlmContextLedger, context_aware_system_user_request};
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

use a4n_prompt::rendering::PromptRenderer;

use super::super::prompt::build_chapter_analysis_stage_user_message;
use crate::source_markers::SourceMarkerMap;

mod source;
use source::{add_chapter_analysis_source_pack, chapter_analysis_dynamic_input};
mod trace;
use trace::chapter_analysis_trace_context;

const CHAPTER_ANALYSIS_SESSION_PROMPT: &str = "chapter_analysis_session";

pub(super) async fn build_chapter_analysis_stage_request(
    ch: &ChapterRecord,
    prompt_renderer: &dyn PromptRenderer,
    prompt_name: &str,
    instruction: &str,
    input: Value,
    output_contract: Value,
) -> Result<ChatRequest, AppError> {
    let variables = HashMap::new();
    let stage_prompt = prompt_renderer
        .render_prompt_for_call(prompt_name, prompt_name, &variables, Some(ch.project_id))
        .await?;
    let system_prompt = prompt_renderer
        .render_prompt_for_call(
            "chapter_analysis::owner::requests::context::build_chapter_analysis_stage_request",
            CHAPTER_ANALYSIS_SESSION_PROMPT,
            &variables,
            Some(ch.project_id),
        )
        .await?;
    let source_spans = source_spans_for_chapters(std::slice::from_ref(ch));
    let source_markers = SourceMarkerMap::from_source_spans(&source_spans)?;
    let dynamic_input = source_markers.encode_input(&chapter_analysis_dynamic_input(input))?;
    let user_prompt = build_chapter_analysis_stage_user_message(
        prompt_name,
        instruction,
        dynamic_input,
        output_contract,
    )?;
    let user_prompt = format!("[阶段指令]\n{}\n\n{user_prompt}", stage_prompt.trim());
    let mut ledger = LlmContextLedger::new();
    add_chapter_analysis_source_pack(&mut ledger, ch, &source_spans, &source_markers)?;
    let manifest = ledger.into_manifest(user_prompt, None);
    Ok(
        context_aware_system_user_request(system_prompt, manifest, Some(0.2), None)?
            .with_prompt_assembly_key(prompt_name)
            .with_json_response_format()
            .with_trace_context(chapter_analysis_trace_context(ch.project_id, prompt_name)),
    )
}
