use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_provider_contract::LlmProviderConfig;
use a4n_queue::priority::TaskPriority;

use super::super::budget::ensure_max_tokens_within_model_cap;
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
        let profile = request.max_tokens_profile.clone();
        request.max_tokens = self.resolve_max_tokens(&request).await?;
        ensure_max_tokens_within_model_cap(
            request.max_tokens,
            profile.as_deref(),
            model_max_output_tokens,
            &model_name,
        )?;
        let output = tokio::time::timeout(a4n_llm::provider::LLM_CALL_TOTAL_TIMEOUT, async {
            let task = self
                .facade
                .queue_start_llm_task(resolved_model.concurrency_limit(), TaskPriority::High, None)
                .await?;
            let cancellation = task.cancellation_token();
            let result = tokio::select! {
                biased;
                _ = cancellation.cancelled() => Err(AppError::Cancelled),
                result = probe_llm_via_sidecar(client, resolved_model, request) => result,
            };
            crate::provider_runtime::finish_provider_task(task, result).await
        })
        .await
        .map_err(|_| AppError::Timeout("LLM 连接测试超过总时限（包含排队）".into()))??;
        tracing::debug!(
            target: "a4n::llm",
            model = %model_name,
            config_id = %config_id,
            profile = ?profile,
            response_preview = %output.response_preview,
            finish_reason = ?output.finish_reason,
            usage = ?output.usage,
            "LLM sidecar probe 响应"
        );
        Ok(())
    }
}
