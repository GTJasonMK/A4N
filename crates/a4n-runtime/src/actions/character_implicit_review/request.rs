use super::{
    CheckImplicitUpdateInput, IMPLICIT_UPDATE_PROMPT_NAME, ImplicitUpdateLlmRequestInput,
    build_implicit_attribute_review, context::implicit_attribute_value,
    context::implicit_review_context_for_user, prompt::build_implicit_update_manifest_user_prompt,
    prompt::implicit_update_stable_context, validate_implicit_review_params,
};
use crate::{
    AppState,
    actions::{
        llm_request::{
            ContextAwareJsonLlmRequestInput, PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT,
            context_aware_system_user_json_llm_request,
        },
        prompt_records::render_prompt_template,
    },
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_llm::provider::ChatRequest;
use serde_json::json;
use std::collections::HashMap;

use super::contracts::CheckImplicitUpdateRequestContext;

pub(super) async fn build_check_implicit_update_request(
    state: &AppState,
    user_id: i64,
    input: &CheckImplicitUpdateInput,
) -> Result<CheckImplicitUpdateRequestContext, String> {
    let attribute_key = input.attribute_key.trim();
    if attribute_key.is_empty() {
        return Err("隐性属性 key 不能为空".to_string());
    }

    let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
        input.threshold,
        input.window_chapters,
        input.evidence_limit,
    )?;

    let (profile, records) =
        implicit_review_context_for_user(state, user_id, input.profile_id).await?;
    let current_value = implicit_attribute_value(&profile.current_state, attribute_key)?;
    let stats = build_implicit_attribute_review(
        attribute_key,
        current_value.clone(),
        &records,
        threshold,
        window_chapters,
        evidence_limit,
    )?;

    let request = build_implicit_update_request_for_state(
        state,
        ImplicitUpdateLlmRequestInput {
            project_id: profile.project_id,
            profile_id: Some(profile.id),
            profile_name: &profile.name,
            attribute_key,
            current_value: &current_value,
            stats: &stats,
            threshold,
            window_chapters,
            source_refs_json: Some(json!({
                "trigger": "manual_review",
                "profile_id": profile.id,
                "attribute_key": attribute_key,
                "window_chapters": window_chapters,
                "evidence_limit": evidence_limit})),
            call_key: Some(format!(
                "implicit_update_review:{}:{attribute_key}",
                profile.id
            )),
        },
    )
    .await?;

    Ok(CheckImplicitUpdateRequestContext {
        project_id: profile.project_id,
        current_value,
        stats,
        request,
    })
}

pub(crate) async fn build_implicit_update_request_for_state(
    state: &AppState,
    input: ImplicitUpdateLlmRequestInput<'_>,
) -> Result<ChatRequest, String> {
    let system_prompt = render_prompt_template(
        state,
        IMPLICIT_UPDATE_PROMPT_NAME,
        HashMap::new(),
        Some(input.project_id),
    )
    .await
    .map_err(|e| format!("渲染 {IMPLICIT_UPDATE_PROMPT_NAME} 提示词失败: {e}"))?;
    build_implicit_update_request_with_system_prompt(system_prompt, input)
}

fn build_implicit_update_request_with_system_prompt(
    system_prompt: String,
    input: ImplicitUpdateLlmRequestInput<'_>,
) -> Result<ChatRequest, String> {
    let dynamic_user_prompt =
        build_implicit_update_manifest_user_prompt(input.profile_name, input.attribute_key)?;
    let stable_context_json = implicit_update_stable_context(
        input.profile_name,
        input.attribute_key,
        input.current_value,
        input.stats,
        input.threshold,
        input.window_chapters,
    );
    let mut source_refs = input.source_refs_json.unwrap_or_else(|| json!({}));
    let Some(source_refs_object) = source_refs.as_object_mut() else {
        return Err("隐性属性复审 source_refs_json 必须是 JSON 对象".to_string());
    };
    source_refs_object.insert("profile_name".to_string(), json!(input.profile_name));
    source_refs_object.insert("attribute_key".to_string(), json!(input.attribute_key));
    if let Some(profile_id) = input.profile_id {
        source_refs_object.insert("profile_id".to_string(), json!(profile_id));
    }
    source_refs_object.insert(
        "evidence_count".to_string(),
        json!(input.stats.evidence.len()),
    );

    context_aware_system_user_json_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id: input.project_id,
            workflow_run_id: None,
            workflow: "implicit_update_review".to_string(),
            step_key: "implicit_update_decision".to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT.to_string(),
            source_refs_json: Some(source_refs),
            config_json: json!({
                "ledger_version": "implicit_update_review_context_v1",
                "prompt_name": IMPLICIT_UPDATE_PROMPT_NAME,
                "threshold": input.threshold,
                "window_chapters": input.window_chapters}),
            stable_context_json,
            dynamic_user_prompt,
            temperature: Some(0.2),
            max_tokens_profile: llm_token_profiles::JSON_STANDARD.to_string(),
            call_key: input.call_key,
        },
    )
}
