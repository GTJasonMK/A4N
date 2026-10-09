use crate::blueprint_generation::stage_response::parse_blueprint_stage_output_schema;
use a4n_domain::project::{BLUEPRINT_GENERATION_KIND, BLUEPRINT_STAGE_SYSTEM_PROMPT};
use a4n_error::AppError;
use a4n_llm_context::context_aware_system_user_request;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use serde_json::json;
use std::collections::HashMap;

use super::super::BlueprintStageRequestInput;
use super::{build_blueprint_stage_context_manifest, build_blueprint_stage_manifest_user_message};

pub async fn build_blueprint_stage_request(
    input: BlueprintStageRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let BlueprintStageRequestInput {
        prompt_renderer,
        project_id,
        base_prompt,
        user_payload_json,
        stage_key,
        stage_name,
        output_schema,
        previous_context,
    } = input;
    let output_schema = parse_blueprint_stage_output_schema(output_schema, stage_name)?;
    let user_content = build_blueprint_stage_manifest_user_message(
        prompt_renderer,
        Some(project_id),
        stage_key,
        stage_name,
        &output_schema,
    )
    .await?;
    let system_content = prompt_renderer
        .render_prompt_for_call(
            "a4n_prompt::blueprint_generation::build_blueprint_stage_request",
            BLUEPRINT_STAGE_SYSTEM_PROMPT,
            &HashMap::from([("base_prompt".to_string(), json!(base_prompt))]),
            Some(project_id),
        )
        .await?;
    let manifest = build_blueprint_stage_context_manifest(
        user_payload_json,
        previous_context,
        user_content,
        stage_key,
    )?;
    Ok(
        context_aware_system_user_request(system_content, manifest, Some(0.7), None)?
            .with_prompt_assembly_key(
                "a4n_prompt::blueprint_generation::build_blueprint_stage_request",
            )
            .with_json_response_format()
            .with_trace_context(LlmTraceContext {
                project_id: Some(project_id),
                root_project_id: None,
                workflow: BLUEPRINT_GENERATION_KIND.to_string(),
                step_key: stage_key.to_string(),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: Some(stage_key.to_string()),
                input_fingerprint_sha256: None,
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
    )
}
