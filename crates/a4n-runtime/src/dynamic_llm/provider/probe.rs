use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_provider_contract::LlmProviderConfig;
use a4n_queue::priority::{TaskKind, TaskPriority};

use super::super::budget::requested_max_tokens;
use super::super::config::probe_llm_via_sidecar;
use super::super::contracts::{DynamicLlmProvider, ResolvedLlmModel};

impl DynamicLlmProvider {
    pub(crate) async fn probe_with_config(
        &self,
        config: LlmProviderConfig,
        mut request: ChatRequest,
    ) -> Result<(), AppError> {
        let config_id = config.id;
        let resolved_model = ResolvedLlmModel::new(config)?;
        let client = self.sidecar_client.clone().ok_or_else(|| {
            AppError::ConfigError(
                "LLM sidecar capability 未配置，无法执行 sidecar connectivity probe".to_string(),
            )
        })?;
        let model_name = resolved_model.model_name().to_string();
        let model_max_output_tokens = resolved_model.capabilities.max_output_tokens;
        request.max_tokens = Some(requested_max_tokens(
            request.max_tokens,
            model_max_output_tokens,
            &model_name,
        )?);
        let output = self
            .facade
            .queue_run_tracked(TaskKind::LlmCall, TaskPriority::High, None, || async move {
                probe_llm_via_sidecar(client, resolved_model, request).await
            })
            .await?;
        tracing::debug!(
            target: "a4n::llm",
            model = %model_name,
            config_id = %config_id,
            response_preview = %output.response_preview,
            finish_reason = ?output.finish_reason,
            usage = ?output.usage,
            "LLM sidecar probe 响应"
        );
        Ok(())
    }
}
