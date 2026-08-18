//! 导入反推 LLM 请求装配。

mod context;
mod contracts;
mod output_contract;
mod payload;
#[cfg(test)]
mod tests;

use a4n_core::error::AppError;
use a4n_llm::context::context_aware_system_user_request;
use a4n_llm::provider::{ChatRequest, LlmTraceContext};

use self::context::build_reverse_engineer_context;
pub use contracts::ReverseEngineerRequestInput;
use payload::{build_reverse_engineer_user_message, reverse_engineer_source_artifact};

use super::IMPORT_REVERSE_WORKFLOW_KIND;

pub async fn build_reverse_engineer_request(
    input: ReverseEngineerRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let ReverseEngineerRequestInput {
        project_id,
        target,
        content_sample,
        prompt_renderer,
        variables,
    } = input;
    let prompt_name = target.prompt_name();
    let max_tokens_profile = target.max_tokens_profile();
    let sys = prompt_renderer
        .render_prompt(prompt_name, variables, Some(project_id))
        .await?;
    let (manifest, fingerprint) = build_reverse_engineer_context(target, content_sample)?;
    Ok(context_aware_system_user_request(
        sys,
        manifest,
        Some(0.3),
        None,
        Some(max_tokens_profile.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: IMPORT_REVERSE_WORKFLOW_KIND.to_string(),
        step_key: prompt_name.to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(prompt_name.to_string()),
        input_fingerprint_sha256: Some(fingerprint),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}
