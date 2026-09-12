use crate::provider_runtime::DynamicProviderFacade;
use a4n_error::AppError;
use a4n_llm::{
    context::{
        ContextBudgetPlanner, LlmContextManifest, LlmContextPackRequirement,
        render_context_manifest_user_content, replace_request_user_content_from_manifest,
        validate_context_manifest,
    },
    provider::{ChatMessage, ChatRequest, LlmProvider, LlmTraceContext},
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
                .count_messages_tokens(&[ChatMessage::user(render_context_manifest_user_content(
                    &manifest,
                    &[],
                )?)])
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
        request.cache_user_prefix_len = None;
        return;
    }

    let Some(project_id) = trace_context.root_project_id.or(trace_context.project_id) else {
        request.cache_user_prefix_len = None;
        return;
    };
    let Some(prefix_sha256) = cacheable_prefix_sha256(request) else {
        request.cache_user_prefix_len = None;
        return;
    };
    let model_sha256 = format!("{:x}", Sha256::digest(model_name.as_bytes()));
    request.cache_scope_key = Some(format!(
        "a4n:project:{project_id}:m{}:p{}",
        &model_sha256[..12],
        &prefix_sha256[..16]
    ));
    request.cache_system_prefix = true;
}

fn cacheable_prefix_sha256(request: &ChatRequest) -> Option<String> {
    let mut hasher = Sha256::new();
    let mut has_prefix = false;
    for message in &request.messages {
        if !matches!(message.role, a4n_llm::provider::MessageRole::System) {
            continue;
        }
        update_cache_hash(&mut hasher, b"system", message.content.as_bytes());
        has_prefix = true;
    }
    if let Some(prefix) = stable_user_prefix(request) {
        update_cache_hash(&mut hasher, b"user", prefix.as_bytes());
        has_prefix = true;
    }
    has_prefix.then(|| format!("{:x}", hasher.finalize()))
}

fn stable_user_prefix(request: &ChatRequest) -> Option<&str> {
    let prefix_len = request.cache_user_prefix_len?;
    let index = request
        .messages
        .iter()
        .rposition(|message| matches!(message.role, a4n_llm::provider::MessageRole::User))?;
    let content = &request.messages[index].content;
    (prefix_len > 0 && prefix_len <= content.len() && content.is_char_boundary(prefix_len))
        .then(|| &content[..prefix_len])
}

fn update_cache_hash(hasher: &mut Sha256, kind: &[u8], content: &[u8]) {
    hasher.update(kind.len().to_le_bytes());
    hasher.update(kind);
    hasher.update(content.len().to_le_bytes());
    hasher.update(content);
}

#[cfg(test)]
mod tests {
    use a4n_llm::provider::{ChatMessage, ChatRequest, LlmTraceContext};

    use super::configure_prompt_cache;

    #[test]
    fn gpt_5_6_cache_reuses_stable_project_prefix_across_runs() {
        let mut request = ChatRequest::system_user("stable", "dynamic", None, None);
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
        assert!(first_key.starts_with("a4n:project:1:m"));
        assert!(first_key.len() <= 64);
        assert!(request.cache_system_prefix);

        request.messages[1].content = "different dynamic input".to_string();
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_eq!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        trace.workflow_run_id = Some(300);
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_eq!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        trace.generation_run_id = Some(301);
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_eq!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        request.messages[0].content = "different stable prefix".to_string();
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        let changed_system_key = request.cache_scope_key.clone().expect("cache key");
        assert_ne!(changed_system_key, first_key);

        request.messages[0].content = "stable".to_string();
        trace.root_project_id = Some(2);
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_ne!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        trace.root_project_id = Some(1);
        configure_prompt_cache(&mut request, "gpt-5.6-other", &trace);
        assert_ne!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        trace.root_project_id = None;
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_eq!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        trace.project_id = None;
        let mut unrelated = ChatRequest::new(vec![ChatMessage::user("one shot")], None, None);
        unrelated.cache_user_prefix_len = Some(1);
        configure_prompt_cache(&mut unrelated, "gpt-5.6-luna", &trace);
        assert!(unrelated.cache_scope_key.is_none());
        assert!(!unrelated.cache_system_prefix);
        assert!(unrelated.cache_user_prefix_len.is_none());
    }

    #[test]
    fn gpt_5_6_cache_key_includes_stable_manifest_user_prefix_only() {
        let mut request = ChatRequest::system_user(
            "stable system",
            "stable context\n\ncurrent task one",
            None,
            None,
        );
        request.cache_user_prefix_len = Some("stable context".len());
        let trace = LlmTraceContext {
            project_id: Some(1),
            root_project_id: Some(1),
            workflow: "manga_pipeline".to_string(),
            step_key: "page".to_string(),
            generation_run_id: Some(1),
            workflow_run_id: Some(1),
            call_key: Some("page:1".to_string()),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        };

        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        let first_key = request.cache_scope_key.clone().expect("cache key");

        request.messages[1].content = "stable context\n\ncurrent task two".to_string();
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_eq!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));

        request.messages[1].content = "different context\n\ncurrent task two".to_string();
        request.cache_user_prefix_len = Some("different context".len());
        configure_prompt_cache(&mut request, "gpt-5.6-luna", &trace);
        assert_ne!(request.cache_scope_key.as_deref(), Some(first_key.as_str()));
    }
}
