use crate::provider_runtime::DynamicProviderFacade;
use a4n_error::AppError;
use a4n_llm_context::{
    ContextBudgetPlanner, LlmContextManifest, LlmContextPackRequirement,
    context_manifest_dynamic_messages, replace_request_user_content_from_manifest,
    validate_context_manifest,
};
use a4n_llm_provider_contract::provider::{ChatRequest, LlmProvider, LlmTraceContext};
use serde_json::json;

use super::super::context::{
    build_manifest_requirements, build_system_prefix_pack_ref,
    hydrate_selected_context_pack_content, non_system_messages, system_messages,
};
use super::complete_llm_trace_project_scope;

pub(in crate::dynamic_llm) async fn prepare_dynamic_llm_trace_context(
    facade: &DynamicProviderFacade,
    provider: &dyn LlmProvider,
    request: &mut ChatRequest,
) -> Result<Option<LlmTraceContext>, AppError> {
    let caps = provider.capabilities();
    let output_reserve = request.max_tokens.unwrap_or(caps.max_output_tokens);
    let tools_tokens = request
        .tools
        .as_ref()
        .map(|tools| serde_json::to_string(tools).map(|text| provider.count_tokens(&text)))
        .transpose()?
        .unwrap_or(0);
    let Some(mut trace_context) = request.trace_context.clone() else {
        ContextBudgetPlanner::default().plan_requirements(
            caps.max_context_tokens,
            output_reserve,
            Vec::new(),
            provider
                .count_messages_tokens(&request.messages)
                .saturating_add(tools_tokens),
        )?;
        return Ok(None);
    };
    complete_llm_trace_project_scope(facade, &mut trace_context).await?;

    let system_messages = system_messages(&request.messages);
    let system_tokens = provider.count_messages_tokens(&system_messages);
    let mut system_requirements = Vec::new();

    let can_record_system_pack = trace_context.project_id.is_some();
    if !system_messages.is_empty() && can_record_system_pack {
        let system_ref =
            build_system_prefix_pack_ref(facade, &trace_context, &system_messages, system_tokens)
                .await?;
        system_requirements.push(LlmContextPackRequirement {
            pack_kind: system_ref.pack_kind.clone(),
            required: true,
            stable_prefix: true,
            variants: vec![system_ref],
        });
    }

    let plan = match request.context_manifest_json.clone() {
        Some(manifest_json) => {
            let manifest: LlmContextManifest =
                serde_json::from_value(manifest_json).map_err(|err| {
                    AppError::InvalidArgument(format!("LLM 上下文 manifest 解析失败: {err}"))
                })?;
            validate_context_manifest(&manifest)?;
            let (mut requirements, content_by_ref) =
                build_manifest_requirements(facade, provider, &trace_context, &manifest).await?;
            let mut all_requirements = system_requirements;
            all_requirements.append(&mut requirements);
            let dynamic_delta_tokens = provider
                .count_messages_tokens(&context_manifest_dynamic_messages(request, &manifest)?)
                .saturating_add(tools_tokens)
                .saturating_add(if can_record_system_pack {
                    0
                } else {
                    system_tokens
                });
            let mut plan = ContextBudgetPlanner::default().plan_requirements(
                caps.max_context_tokens,
                output_reserve,
                all_requirements,
                dynamic_delta_tokens,
            )?;
            hydrate_selected_context_pack_content(&mut plan.selected_packs, &content_by_ref);
            replace_request_user_content_from_manifest(request, &manifest, &plan.selected_packs)?;
            plan
        }
        None => {
            let non_system_messages = non_system_messages(&request.messages);
            let task_delta_tokens = provider
                .count_messages_tokens(&non_system_messages)
                .saturating_add(tools_tokens)
                .saturating_add(if can_record_system_pack {
                    0
                } else {
                    system_tokens
                });
            ContextBudgetPlanner::default().plan_requirements(
                caps.max_context_tokens,
                output_reserve,
                system_requirements,
                task_delta_tokens,
            )?
        }
    };
    trace_context.context_plan_json =
        Some(serde_json::to_value(&plan).map_err(AppError::Serialization)?);
    trace_context.context_pack_refs_json = Some(json!({
        "schema_version": 1,
        "selected": plan.selected_pack_refs,
        "dropped_optional": plan.dropped_optional_pack_refs,
    }));
    request.trace_context = Some(trace_context.clone());
    Ok(Some(trace_context))
}
