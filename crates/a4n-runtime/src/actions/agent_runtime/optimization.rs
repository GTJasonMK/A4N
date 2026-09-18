use super::*;
use crate::actions::agent_domains::content_optimization as domain;
use a4n_agent_runtime_contract::AgentExecutionSnapshot;
use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_llm_provider_contract::provider::validated_model_output_tokens;

pub(super) async fn start(
    state: &AppState,
    context: &AgentRunContext,
    arguments: Value,
    execution: Option<&AgentExecutionSnapshot>,
) -> Result<GenerationTaskHandle, AppError> {
    let request_id = execution
        .and_then(|e| e.resource_preconditions_json.as_ref())
        .and_then(|v| v.get("generation_start_request_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("内容优化启动缺少持久幂等 ID".into()))?
        .to_string();
    let input = parse_arguments(arguments, "send_content_optimization_message")?;
    let mut snapshot = state.agent_runtime_snapshot(context.run.id).await;
    let config_id = context
        .turn
        .model_override_json
        .as_ref()
        .and_then(|v| v.get("model_config_id"))
        .and_then(Value::as_i64)
        .or(context.default_model_config_id);
    if let Some(config_id) = config_id {
        let snapshot = snapshot
            .as_mut()
            .ok_or_else(|| AppError::InvalidArgument("指定优化模型需要用户配置快照".into()))?;
        let selected = snapshot
            .llm
            .iter()
            .chain(snapshot.llm_configs.iter())
            .find(|c| c.id == config_id)
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("配置快照不包含优化模型 #{config_id}"))
            })?;
        snapshot.llm = Some(selected);
    }
    crate::runtime_config::request_scope(context.owner_user_id, snapshot, async {
        let capabilities = state
            .dynamic_llm_provider_for_profile()?
            .capabilities_for_config_id(None)
            .await?;
        domain::send_message(
            state,
            context.owner_user_id,
            &context.scope,
            input,
            request_id,
            validated_model_output_tokens(&capabilities)?,
        )
        .await
    })
    .await
}
