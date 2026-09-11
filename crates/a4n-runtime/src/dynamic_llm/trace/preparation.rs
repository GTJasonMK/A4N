use crate::provider_runtime::DynamicProviderFacade;
use a4n_error::AppError;
use a4n_llm::{
    context::{
        ContextBudgetPlanner, LlmContextManifest, LlmContextPackRequirement,
        render_context_manifest_user_content, replace_request_user_content_from_manifest,
        validate_context_manifest,
    },
    provider::{ChatRequest, LlmProvider, LlmTraceContext},
};
use sha2::{Digest, Sha256};

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
                .count_tokens(&render_context_manifest_user_content(&manifest, &[])?)
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
    trace_context.context_pack_refs_json =
        Some(serde_json::to_value(&plan.selected_pack_refs).map_err(AppError::Serialization)?);
    configure_prompt_cache(request, provider.model_name(), &trace_context);
    request.trace_context = Some(trace_context.clone());
    Ok(Some(trace_context))
}

fn configure_prompt_cache(
    request: &mut ChatRequest,
    model_name: &str,
    trace_context: &LlmTraceContext,
) {
    request.cache_scope_key = None;
    request.cache_system_prefix = false;
    if !model_name.starts_with("gpt-5.6") {
        return;
    }
    let run_scope = trace_context
        .generation_run_id
        .map(|id| format!("a4n:generation_run:{id}"))
        .or_else(|| {
            trace_context
                .workflow_run_id
                .map(|id| format!("a4n:workflow_run:{id}"))
        });
    request.cache_scope_key = run_scope
        .zip(system_prefix_sha256(request))
        .map(|(run_scope, prefix_sha256)| format!("{run_scope}:prefix:{}", &prefix_sha256[..16]));
    request.cache_system_prefix = request.cache_scope_key.is_some();
}

fn system_prefix_sha256(request: &ChatRequest) -> Option<String> {
    let mut hasher = Sha256::new();
    let mut count = 0_usize;
    for message in &request.messages {
        if !matches!(message.role, a4n_llm::provider::MessageRole::System) {
            continue;
        }
        count += 1;
        hasher.update(message.content.len().to_le_bytes());
        hasher.update(message.content.as_bytes());
    }
    (count > 0).then(|| format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use a4n_llm::provider::{ChatMessage, ChatRequest, LlmTraceContext};

    use super::configure_prompt_cache;

    #[test]
    fn gpt_5_6_cache_affinity_is_scoped_to_the_run_and_system_prefix() {
        let mut request = ChatRequest::system_user("stable", "dynamic", None, None, None);
        let mut trace = LlmTraceContext {
            project_id: Some(1),
            root_project_id: Some(1),
            workflow: "manga_pipeline".to_string(),
            step_key: "page".to_string(),
            generation_run_id: None,
            workflow_run_id: Some(299),
            call_key: Some("page:1".to_string()),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        };

        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        let first_key = request.cache_scope_key.clone().expect("cache key");
        assert!(first_key.starts_with("a4n:workflow_run:299:prefix:"));
        assert!(request.cache_system_prefix);

        request.messages[1].content = "different dynamic input".to_string();
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_eq!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        request.messages[0].content = "different session".to_string();
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_ne!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        trace.workflow_run_id = None;
        let mut unrelated = ChatRequest::new(vec![ChatMessage::user("one shot")], None, None, None);
        configure_prompt_cache(&mut unrelated, "gpt-5.6-luna", &trace);
        assert!(unrelated.cache_scope_key.is_none());
        assert!(!unrelated.cache_system_prefix);
    }
}
