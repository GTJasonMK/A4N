use super::super::shared::validate_runtime_snapshot_config_id;
use super::contracts::{positive_model_context_cap, positive_model_output_cap};
use crate::actions::request_preview::{
    ChatRequestStepInput, ChatRequestStepWithBudgetInput, ModelRequestPreview,
    chat_request_step_with_budget, model_request_preview,
};
use crate::{AppState, actions::app_error_message};
use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest};
use a4n_provider_contract::LlmProviderConfig;

const LLM_CONNECTIVITY_TEST_PROMPT: &str = "ping";

pub async fn test_llm_config(state: &AppState, user_id: i64, config_id: i64) -> Result<(), String> {
    let config = resolve_llm_config_for_connectivity_test(state, user_id, config_id).await?;
    let request = build_llm_connectivity_test_request(&config)?;
    let provider = state
        .dynamic_llm_provider_for_profile()
        .map_err(|error| error.to_string())?;
    if state.runtime_profile().uses_sidecars() {
        return provider
            .probe_with_config(config.clone(), request)
            .await
            .map_err(|error| error.to_string());
    }
    let response = provider
        .chat_with_config(config, request)
        .await
        .map_err(|error| error.to_string())?;
    if response.message.content.trim().is_empty() {
        return Err("LLM 连接测试返回空响应".to_string());
    }
    Ok(())
}

pub async fn preview_test_llm_config_request(
    state: &AppState,
    user_id: i64,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_llm_config_for_connectivity_test(state, user_id, config_id).await?;
    let request = build_llm_connectivity_test_request(&config)?;
    let context_cap = positive_model_context_cap(config.max_context_tokens)?;
    let output_cap = positive_model_output_cap(config.max_output_tokens)?;
    Ok(model_request_preview(
        "llm_config_connectivity_test",
        "LLM 配置测试请求预览",
        vec![chat_request_step_with_budget(
            ChatRequestStepWithBudgetInput {
                step: ChatRequestStepInput {
                    key: "llm_connectivity_test".to_string(),
                    title: "LLM 连接测试请求".to_string(),
                    prompt_name: None,
                    state: "ready".to_string(),
                    note: Some("固定连接测试请求：ping".to_string()),
                    request,
                },
                model_context_tokens: context_cap,
                output_token_reserve: output_cap,
                budget_note: Some(format!(
                    "被测配置模型={}；上下文={}；输出上限={}",
                    config.model, context_cap, output_cap
                )),
            },
        )?],
    ))
}

async fn resolve_llm_config_for_connectivity_test(
    state: &AppState,
    user_id: i64,
    config_id: i64,
) -> Result<LlmProviderConfig, String> {
    if let Some(config) = crate::runtime_config::current_llm_provider_config() {
        validate_llm_runtime_snapshot_config_id(&config, config_id)?;
        return Ok(config);
    }
    state
        .config_read_application()
        .llm_config_for_connectivity_test(user_id, config_id)
        .await
        .map_err(app_error_message)
}

fn build_llm_connectivity_test_request(
    config: &a4n_provider_contract::LlmProviderConfig,
) -> Result<ChatRequest, String> {
    let max_tokens = 8.min(positive_model_output_cap(config.max_output_tokens)?);
    Ok(ChatRequest::new(
        vec![ChatMessage::user(LLM_CONNECTIVITY_TEST_PROMPT)],
        Some(0.0),
        Some(max_tokens),
    ))
}

pub(super) fn validate_llm_runtime_snapshot_config_id(
    config: &a4n_provider_contract::LlmProviderConfig,
    config_id: i64,
) -> Result<(), String> {
    validate_runtime_snapshot_config_id("llm", config.id, config_id)
}
