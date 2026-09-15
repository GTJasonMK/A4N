mod hydration;
mod requirements;

use super::super::contracts::{LlmContextManifest, LlmContextRequestPreview};
use super::super::manifest::validate_context_manifest;
use super::super::planner::{ContextBudgetPlanner, estimate_context_tokens};
use super::{context_manifest_dynamic_messages, replace_request_user_content_from_manifest};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, MessageRole};
use hydration::hydrate_selected_pack_content;
use requirements::preview_requirement_inputs;

pub fn preview_context_aware_chat_request(
    request: &mut ChatRequest,
    model_context_tokens: usize,
    output_token_reserve: usize,
) -> Result<Option<LlmContextRequestPreview>, AppError> {
    let Some(manifest_json) = request.context_manifest_json.clone() else {
        return Ok(None);
    };
    let manifest: LlmContextManifest = serde_json::from_value(manifest_json.clone())
        .map_err(|err| AppError::InvalidArgument(format!("LLM 上下文 manifest 解析失败: {err}")))?;
    validate_context_manifest(&manifest)?;
    let system_messages = request
        .messages
        .iter()
        .filter(|message| matches!(message.role, MessageRole::System))
        .cloned()
        .collect::<Vec<_>>();
    let requirement_inputs = preview_requirement_inputs(&system_messages, &manifest)?;
    let tools_tokens = request
        .tools
        .as_ref()
        .map(|tools| serde_json::to_string(tools).map(|text| estimate_context_tokens(&text)))
        .transpose()?
        .unwrap_or(0);
    let dynamic_delta_tokens = estimate_context_tokens(&serde_json::to_string(
        &context_manifest_dynamic_messages(request, &manifest)?,
    )?)
    .saturating_add(tools_tokens);
    let mut plan = ContextBudgetPlanner::default().plan_requirements_adaptive(
        model_context_tokens,
        output_token_reserve,
        requirement_inputs.requirements,
        dynamic_delta_tokens,
    )?;
    hydrate_selected_pack_content(&mut plan, &requirement_inputs.content_by_key);
    replace_request_user_content_from_manifest(request, &manifest, &plan.selected_packs)?;
    request.max_tokens = Some(plan.output_token_reserve);
    Ok(Some(LlmContextRequestPreview {
        manifest_json,
        plan_json: serde_json::to_value(&plan).map_err(AppError::Serialization)?,
        pack_refs_json: serde_json::to_value(&plan.selected_pack_refs)
            .map_err(AppError::Serialization)?,
        stable_prefix_tokens: plan.stable_prefix_tokens,
        dynamic_delta_tokens: plan.dynamic_delta_tokens,
        available_input_tokens: plan.available_input_tokens,
    }))
}
