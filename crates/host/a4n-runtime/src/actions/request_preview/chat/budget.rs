use super::super::contracts::{
    ChatRequestStepInput, ChatRequestStepWithBudgetInput, ModelRequestPreviewStep,
};
use super::chat_request_step_with_budget;
use crate::{AppState, dynamic_llm, runtime_config};
use a4n_provider_contract::LlmProviderConfig;

pub(crate) async fn chat_request_step_for_state(
    state: &AppState,
    mut step: ChatRequestStepInput,
) -> Result<ModelRequestPreviewStep, String> {
    if step.prompt_assembly.is_none()
        && let (Some(call_key), Some(prompt_name)) = (
            step.request.prompt_assembly_key.as_deref(),
            step.prompt_name.as_deref(),
        )
    {
        let project_id = step
            .request
            .trace_context
            .as_ref()
            .and_then(|context| context.project_id);
        let context = crate::actions::prompt_records::prompt_context(state);
        step.prompt_assembly = Some(
            state
                .prompt_read_application()
                .resolve_assembly_record(&context, call_key, prompt_name, project_id)
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    let model = resolve_preview_llm_model_for_state(state).await?;
    let budget = dynamic_llm::resolve_effective_llm_preview_budget_for_config(model, &step.request)
        .map_err(|err| err.to_string())?;
    chat_request_step_with_budget(ChatRequestStepWithBudgetInput {
        step,
        model_context_tokens: budget.max_context_tokens,
        output_token_reserve: budget.output_token_reserve,
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
) -> Result<LlmProviderConfig, String> {
    if let Some(model) = runtime_config::current_llm_provider_config() {
        return Ok(model);
    }
    if runtime_config::uses_system_default_llm_template() {
        let mut model = state
            .provider_runtime()
            .resolve_llm_config(None)
            .await
            .map_err(|err| err.to_string())?;
        model.api_key = None;
        return Ok(model);
    }
    if !state.provider_runtime().allows_host_provider_config() {
        return Err(
            "LLM 请求预览需要用户 LLM 配置快照或系统默认模板；请先在用户配置页选择默认 LLM"
                .to_string(),
        );
    }
    state
        .provider_runtime()
        .resolve_llm_config(None)
        .await
        .map_err(|err| err.to_string())
}
