use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_queue::priority::{TaskPriority, TaskType};

use super::super::budget::{ensure_max_tokens_within_model_cap, llm_probe_succeeded};
use super::super::config::probe_llm_via_sidecar;
use super::super::contracts::DynamicLlmProvider;

impl DynamicLlmProvider {
    pub async fn probe_with_config_id(
        &self,
        config_id: Option<i64>,
        mut request: ChatRequest,
    ) -> Result<bool, AppError> {
        let client = self.sidecar_client.clone().ok_or_else(|| {
            AppError::ConfigError(
                "LLM sidecar capability 未配置，无法执行 sidecar connectivity probe".to_string(),
            )
        })?;
        let resolved_model = self.resolve_runtime_model_by_id(config_id).await?;
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
        let output = self
            .task_queue
            .run_tracked(TaskType::LlmCall, TaskPriority::High, None, || async move {
                probe_llm_via_sidecar(client, resolved_model, request).await
            })
            .await?;
        let succeeded = llm_probe_succeeded(&output);
        tracing::debug!(
            target: "a4n::llm",
            model = %model_name,
            config_id = ?config_id,
            profile = ?profile,
            ok = output.ok,
            finish_reason = ?output.finish_reason,
            usage = ?output.usage,
            "LLM sidecar probe 响应"
        );
        Ok(succeeded)
    }
}
