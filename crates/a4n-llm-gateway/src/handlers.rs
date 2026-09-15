mod discovery;
mod probe;
mod stream;

use a4n_domain::{DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS};
use a4n_error::AppError;
use a4n_llm::provider::{LlmProvider, ModelCapabilities};
use a4n_llm_sidecar_contract::LlmGatewayProviderConfig;
use a4n_service_protocol::{HealthStatus, LLM_SERVICE_NAME, ServiceHealth, known_service_health};
use axum::Json;

pub(crate) use discovery::discover_models;
pub(crate) use probe::probe;
#[cfg(test)]
pub(crate) use probe::probe_existing_provider;
#[cfg(test)]
pub(crate) use stream::llm_event_stream;
pub(crate) use stream::stream_chat;

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        LLM_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}

fn provider_from_config(
    config: LlmGatewayProviderConfig,
) -> Result<Box<dyn LlmProvider>, AppError> {
    a4n_llm_http::pick_llm_provider(
        &config.provider,
        config.base_url,
        config.model,
        config.api_key,
        config.timeout_secs,
        config.extra_body_json,
        ModelCapabilities {
            max_context_tokens: DEFAULT_LLM_MAX_CONTEXT_TOKENS,
            max_output_tokens: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
        },
    )
}
