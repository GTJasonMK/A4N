use a4n_domain::cost::{
    ModelCapability, ModelUsageMeter, ModelUsageRecordCompletion, ModelUsageRecordStart,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, LLM_CALL_TOTAL_TIMEOUT};
use a4n_provider_contract::LlmProviderConfig;
use a4n_queue::priority::TaskPriority;

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
        let cost_record = self
            .facade
            .start_model_usage(ModelUsageRecordStart {
                request_id: uuid::Uuid::new_v4().to_string(),
                user_id: crate::runtime_config::current_user_id(),
                project_id: None,
                root_project_id: None,
                workflow_run_id: None,
                generation_run_id: None,
                workflow: "llm_connectivity_test".to_string(),
                step_key: "probe".to_string(),
                call_key: None,
                source: Some("connection_test".to_string()),
                capability: ModelCapability::Llm,
                provider_config_id: Some(resolved_model.model.id),
                provider: Some(resolved_model.model.provider.clone()),
                model: Some(resolved_model.model.model.clone()),
                pricing: resolved_model.model.pricing.clone(),
                request_json: Some(a4n_llm_http::request_body_preview(
                    &resolved_model.model.provider,
                    &resolved_model.model.model,
                    resolved_model.model.extra_body_json.clone(),
                    resolved_model.capabilities.clone(),
                    &request,
                )?),
            })
            .await?;
        let probe_result = match tokio::time::timeout(LLM_CALL_TOTAL_TIMEOUT, async {
            let task = match self
                .facade
                .queue_start_llm_task(resolved_model.concurrency_limit(), TaskPriority::High, None)
                .await
            {
                Ok(task) => task,
                Err(error) => return Err(error),
            };
            let cancellation = task.cancellation_token();
            let result = tokio::select! {
                biased;
                _ = cancellation.cancelled() => Err(AppError::Cancelled),
                result = probe_llm_via_sidecar(client, resolved_model, request) => result,
            };
            crate::provider_runtime::finish_provider_task(task, result).await
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                let error = AppError::Timeout("LLM 连接测试超过总时限（包含排队）".into());
                self.facade
                    .finish_model_usage(
                        cost_record,
                        ModelUsageRecordCompletion {
                            status: "failed".to_string(),
                            meter: Default::default(),
                            error_kind: Some("timeout".to_string()),
                            error_message: Some(error.to_string()),
                        },
                    )
                    .await?;
                return Err(error);
            }
        };
        let output = match probe_result {
            Ok(output) => output,
            Err(error) => {
                self.facade
                    .finish_model_usage(
                        cost_record,
                        ModelUsageRecordCompletion {
                            status: if matches!(error, AppError::Cancelled) {
                                "cancelled".to_string()
                            } else {
                                "failed".to_string()
                            },
                            meter: Default::default(),
                            error_kind: Some("provider".to_string()),
                            error_message: Some(error.to_string()),
                        },
                    )
                    .await?;
                return Err(error);
            }
        };
        self.facade
            .finish_model_usage(
                cost_record,
                ModelUsageRecordCompletion {
                    status: "completed".to_string(),
                    meter: output
                        .usage
                        .as_ref()
                        .map(|usage| ModelUsageMeter {
                            input_tokens: Some(usage.prompt_tokens as u64),
                            output_tokens: Some(usage.completion_tokens as u64),
                            cache_read_tokens: usage
                                .prompt_cache_hit_tokens
                                .map(|value| value as u64),
                            cache_write_tokens: usage
                                .prompt_cache_write_tokens
                                .map(|value| value as u64),
                            ..Default::default()
                        })
                        .unwrap_or_default(),
                    error_kind: None,
                    error_message: None,
                },
            )
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
