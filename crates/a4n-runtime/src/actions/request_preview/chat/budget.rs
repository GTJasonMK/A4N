use super::super::contracts::{
    ChatRequestStepInput, ChatRequestStepWithBudgetInput, ModelRequestPreviewStep,
};
use super::chat_request_step_with_budget;
use crate::{AppState, dynamic_llm, runtime_config};
use a4n_core::config::app_config::LlmMaxTokensConfig;
use a4n_core::models::ai::llm_config;
use a4n_llm::provider::ChatRequest;

pub(crate) async fn chat_request_step_for_state(
    state: &AppState,
    step: ChatRequestStepInput,
) -> Result<ModelRequestPreviewStep, String> {
    let model = resolve_preview_llm_model_for_state(state).await?;
    let max_tokens_config =
        resolve_preview_llm_max_tokens_config_for_state(state, &step.request).await?;
    let budget = dynamic_llm::resolve_effective_llm_preview_budget_for_config(
        model,
        &step.request,
        max_tokens_config.as_ref(),
    )
    .map_err(|err| err.to_string())?;
    chat_request_step_with_budget(ChatRequestStepWithBudgetInput {
        step,
        model_context_tokens: budget.max_context_tokens,
        output_token_reserve_override: Some(budget.output_token_reserve),
        budget_note: Some(format!(
            "预览模型={}；上下文={}；输出上限={}；输出保留={}（{}）",
            budget.model_name,
            budget.max_context_tokens,
            budget.max_output_tokens,
            budget.output_token_reserve,
            budget.output_token_source
        )),
    })
}

async fn resolve_preview_llm_model_for_state(
    state: &AppState,
) -> Result<llm_config::Model, String> {
    if let Some(model) = runtime_config::current_llm_model() {
        return Ok(model);
    }
    if runtime_config::uses_system_default_llm_template() {
        let mut model = state
            .settings()
            .resolve_llm_config(None)
            .await
            .map_err(|err| err.to_string())?;
        model.api_key_encrypted = None;
        return Ok(model);
    }
    if runtime_config::current_user_id().is_some() {
        return Err(
            "LLM 请求预览需要用户 LLM 配置快照或系统默认模板；请先在用户配置页选择默认 LLM"
                .to_string(),
        );
    }
    state
        .settings()
        .resolve_llm_config(None)
        .await
        .map_err(|err| err.to_string())
}

async fn resolve_preview_llm_max_tokens_config_for_state(
    state: &AppState,
    request: &ChatRequest,
) -> Result<Option<LlmMaxTokensConfig>, String> {
    if request.max_tokens_profile.is_none() {
        return Ok(None);
    }
    match runtime_config::current_llm_max_tokens() {
        Some(config) => Ok(Some(config)),
        None => state
            .settings()
            .llm_max_tokens_config()
            .await
            .map(Some)
            .map_err(|err| err.to_string()),
    }
}
