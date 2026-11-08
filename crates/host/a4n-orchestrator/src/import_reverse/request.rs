//! 导入反推 LLM 请求装配。

mod context;
mod contracts;
mod output_contract;
mod payload;
#[cfg(test)]
mod tests;

use a4n_error::AppError;
use a4n_llm_context::context_aware_system_user_request;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};

use self::context::build_reverse_engineer_context;
pub(in crate::import_reverse) use contracts::ReverseEngineerRequestInput;
use payload::{build_reverse_engineer_user_message, reverse_engineer_source_artifact};

use super::IMPORT_REVERSE_WORKFLOW_KIND;

pub(in crate::import_reverse) async fn build_reverse_engineer_request(
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
    let sys = prompt_renderer
        .render_prompt_for_call(
            "ImportService.reverse_engineer",
            prompt_name,
            variables,
            Some(project_id),
        )
        .await?;
    let (manifest, fingerprint) = build_reverse_engineer_context(target, content_sample)?;
    Ok(
        context_aware_system_user_request(sys, manifest, Some(0.3), None)?
            .with_prompt_assembly_key("ImportService.reverse_engineer")
            .with_json_schema_response_format(
                prompt_name,
                output_contract::reverse_output_schema(target),
                false,
            )?
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
            }),
    )
}
