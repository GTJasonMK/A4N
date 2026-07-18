use std::sync::Arc;

use a4n_capabilities::A4nCapabilities;
use a4n_core::error::AppError;
use a4n_llm::provider::LlmProvider;
use a4n_model_discovery::{ModelDiscoveryRequest, ModelDiscoveryResult};
use a4n_orchestrator::OrchestratorContext;
use a4n_orchestrator::task_queue::PersistentTaskQueue;

use crate::state::RuntimeProfile;

use super::request_context::global_provider_service_request;
use super::response::{response_payload, service_error_string};

pub(crate) fn create_llm_provider(
    task_queue: Arc<PersistentTaskQueue>,
    orchestrator: OrchestratorContext,
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
) -> Result<Arc<dyn LlmProvider>, AppError> {
    let sidecar_client = required_sidecar_client(profile, capabilities.llm.as_ref(), "LLM")?;
    if let Some(client) = sidecar_client {
        return Ok(Arc::new(
            crate::dynamic_llm::DynamicLlmProvider::new_with_sidecar_client(
                orchestrator,
                task_queue,
                client,
            ),
        ));
    }

    Ok(Arc::new(crate::dynamic_llm::DynamicLlmProvider::new(
        orchestrator,
        task_queue,
    )))
}

pub(super) fn required_sidecar_client<T: Clone>(
    profile: RuntimeProfile,
    client: Option<&T>,
    capability: &str,
) -> Result<Option<T>, AppError> {
    if profile.uses_sidecars() {
        return client
            .cloned()
            .map(Some)
            .ok_or_else(|| AppError::ConfigError(missing_sidecar_capability_message(capability)));
    }
    Ok(None)
}

pub(super) fn missing_sidecar_capability_message(capability: &str) -> String {
    format!("{capability} sidecar capability 未配置，无法启用 sidecar profile")
}

pub(crate) async fn discover_llm_models(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<ModelDiscoveryResult, String> {
    if profile.uses_sidecars() {
        let client = capabilities
            .llm
            .clone()
            .ok_or_else(|| "LLM sidecar capability 未配置，无法探测模型列表".to_string())?;
        let response = client
            .discover_models(global_provider_service_request(
                "llm",
                a4n_service_protocol::LLM_CHAT_CAPABILITY_VERSION,
                model_discovery_payload(provider, base_url, api_key),
            )?)
            .await
            .map_err(service_error_string)?;
        return response_payload(response, "llm");
    }

    a4n_model_discovery::discover_models(provider, base_url, api_key).await
}

pub(super) fn model_discovery_payload(
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> ModelDiscoveryRequest {
    ModelDiscoveryRequest {
        provider: provider.to_string(),
        base_url: base_url.to_string(),
        api_key: api_key.map(str::to_string),
    }
}
