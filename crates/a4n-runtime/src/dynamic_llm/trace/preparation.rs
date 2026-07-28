use a4n_core::error::AppError;
use a4n_llm::{
    context::{
        ContextBudgetPlanner, LlmContextManifest, LlmContextPackRequirement,
        render_context_manifest_user_content, replace_request_user_content_from_manifest,
        validate_context_manifest,
    },
    provider::{ChatRequest, LlmProvider, LlmTraceContext},
};
use a4n_orchestrator::DynamicProviderFacade;

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
    let Some(mut trace_context) = request.trace_context.clone() else {
        return Ok(None);
    };
    complete_llm_trace_project_scope(facade, &mut trace_context).await?;

    let caps = provider.capabilities();
    let output_reserve = request.max_tokens.unwrap_or(caps.max_output_tokens);
    let system_messages = system_messages(&request.messages);
    let system_tokens = provider.count_messages_tokens(&system_messages);
    let mut system_requirements = Vec::new();

    if !system_messages.is_empty() {
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
            let dynamic_delta_tokens =
                provider.count_tokens(&render_context_manifest_user_content(&manifest, &[])?);
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
            let task_delta_tokens = provider.count_messages_tokens(&non_system_messages);
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
    trace_context.context_pack_refs_json =
        Some(serde_json::to_value(&plan.selected_pack_refs).map_err(AppError::Serialization)?);
    request.trace_context = Some(trace_context.clone());
    Ok(Some(trace_context))
}
