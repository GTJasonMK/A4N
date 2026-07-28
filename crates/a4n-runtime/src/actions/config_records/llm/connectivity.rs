use super::super::shared::validate_runtime_snapshot_config_id;
use super::contracts::{positive_model_context_cap, positive_model_output_cap};
use super::max_tokens::load_effective_llm_max_tokens_config;
use crate::actions::request_preview::{
    ChatRequestStepInput, ChatRequestStepWithBudgetInput, ModelRequestPreview,
    chat_request_step_with_budget, model_request_preview,
};
use crate::{AppState, actions::app_error_message};
use a4n_core::models::ai::llm_config;
use a4n_llm::provider::{ChatMessage, ChatRequest};

const LLM_CONNECTIVITY_TEST_PROMPT: &str = "ping";

pub async fn test_llm_config(state: &AppState, config_id: i64) -> Result<bool, String> {
    let config = resolve_llm_config_for_connectivity_test(state, config_id).await?;
    let request = build_llm_connectivity_test_request(state, &config).await?;
    let provider = state
        .dynamic_llm_provider_for_profile()
        .map_err(|error| error.to_string())?;
    if state.runtime_profile().uses_sidecars() {
        return provider
            .probe_with_config_id(Some(config_id), request)
            .await
            .map_err(|error| error.to_string());
    }
    let response = provider
        .chat_with_config_id(Some(config_id), request)
        .await
        .map_err(|error| error.to_string())?;
    Ok(!response.message.content.is_empty())
}

pub async fn preview_test_llm_config_request(
    state: &AppState,
    config_id: i64,
) -> Result<ModelRequestPreview, String> {
    let config = resolve_llm_config_for_connectivity_test(state, config_id).await?;
    let request = build_llm_connectivity_test_request(state, &config).await?;
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
                output_token_reserve_override: Some(output_cap),
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
    config_id: i64,
) -> Result<llm_config::Model, String> {
    if let Some(config) = crate::runtime_config::current_llm_model() {
        validate_llm_runtime_snapshot_config_id(&config, config_id)?;
        return Ok(config);
    }
    state
        .settings()
        .llm_config_record_by_id(config_id)
        .await
        .map_err(app_error_message)
}

async fn build_llm_connectivity_test_request(
    state: &AppState,
    config: &llm_config::Model,
) -> Result<ChatRequest, String> {
    let max_tokens = load_effective_llm_max_tokens_config(state)
        .await?
        .connection_test;
    validate_llm_connection_test_max_tokens(max_tokens, config)?;
    Ok(ChatRequest::new(
        vec![ChatMessage::user(LLM_CONNECTIVITY_TEST_PROMPT)],
        Some(0.0),
        Some(max_tokens),
        None,
    ))
}

pub(super) fn validate_llm_runtime_snapshot_config_id(
    config: &llm_config::Model,
    config_id: i64,
) -> Result<(), String> {
    validate_runtime_snapshot_config_id("llm", config.id, config_id)
}

pub(super) fn validate_llm_connection_test_max_tokens(
    max_tokens: usize,
    config: &llm_config::Model,
) -> Result<(), String> {
    let model_cap = positive_model_output_cap(config.max_output_tokens)?;
    if max_tokens <= model_cap {
        return Ok(());
    }
    Err(format!(
        "LLM 连接测试 max_tokens 不能超过被测配置模型最大输出：config_id={}, model={}, connection_test={}, model_max_output_tokens={}",
        config.id, config.model, max_tokens, model_cap
    ))
}
