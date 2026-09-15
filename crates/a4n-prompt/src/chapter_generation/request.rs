//! 章节生成 ChatRequest 装配。

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use serde_json::json;

use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm_context::context_aware_system_user_request;

use super::manifest::build_chapter_generation_context_manifest;
use super::message::{
    build_chapter_generation_user_message, normalize_context_before_section,
    required_generation_outline_text,
};

#[cfg(test)]
mod tests;

pub fn build_chapter_generation_request(
    project_id: i64,
    chapter_number: i32,
    system_prompt: String,
    chapter_outline: &str,
    context_before: Option<&str>,
) -> Result<ChatRequest, AppError> {
    let user_content = build_chapter_generation_user_message(chapter_outline, context_before)?;
    let manifest =
        build_chapter_generation_context_manifest(chapter_outline, context_before, user_content)?;
    Ok(
        context_aware_system_user_request(system_prompt, manifest, Some(0.8), None)?
            .with_trace_context(LlmTraceContext {
                project_id: Some(project_id),
                root_project_id: None,
                workflow: "chapter_generation".to_string(),
                step_key: format!("chapter_{chapter_number}"),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: Some(format!("chapter_{chapter_number}")),
                input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(
                    &json!({
                        "chapter_number": chapter_number,
                        "chapter_outline": required_generation_outline_text(chapter_outline)?,
                        "context_before": normalize_context_before_section(context_before),
                    }),
                )?)),
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
    )
}
