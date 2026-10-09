use std::collections::HashMap;

use a4n_domain::{
    project::{
        BLUEPRINT_REFINEMENT_PROMPT, BLUEPRINT_REFINEMENT_WORKFLOW_KIND,
        blueprint_refinement_call_key,
    },
    source_text_hash as source_content_sha256,
};
use a4n_error::AppError;
use a4n_llm_context::context_aware_system_user_request;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use serde_json::{Value, json};

use crate::rendering::PromptRenderer;

use super::{
    context::build_blueprint_refine_context_manifest,
    user_message::blueprint_refine_manifest_user_message,
};

pub async fn build_blueprint_refine_request(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    focus: Option<&str>,
    variables: &HashMap<String, Value>,
    blueprint_context: &Value,
    inspiration_context: &Value,
) -> Result<ChatRequest, AppError> {
    let system_prompt = prompt_renderer
        .render_prompt_for_call(
            "a4n_prompt::blueprint_generation::build_blueprint_refine_request",
            BLUEPRINT_REFINEMENT_PROMPT,
            variables,
            Some(project_id),
        )
        .await?;
    let user_prompt =
        blueprint_refine_manifest_user_message(prompt_renderer, Some(project_id), focus).await?;
    let dynamic_user_content_sha256 = source_content_sha256(&user_prompt);
    let manifest = build_blueprint_refine_context_manifest(
        blueprint_context,
        inspiration_context,
        user_prompt,
    )?;
    let call_key = blueprint_refinement_call_key(project_id);
    Ok(
        context_aware_system_user_request(system_prompt, manifest, Some(0.6), None)?
            .with_prompt_assembly_key(
                "a4n_prompt::blueprint_generation::build_blueprint_refine_request",
            )
            .with_json_response_format()
            .with_trace_context(LlmTraceContext {
                project_id: Some(project_id),
                root_project_id: None,
                workflow: BLUEPRINT_REFINEMENT_WORKFLOW_KIND.to_string(),
                step_key: BLUEPRINT_REFINEMENT_WORKFLOW_KIND.to_string(),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: Some(call_key),
                input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(
                    &json!({
                        "blueprint_context": blueprint_context,
                        "inspiration_context": inspiration_context,
                        "dynamic_user_content_sha256": dynamic_user_content_sha256,
                    }),
                )?)),
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
    )
}
