mod discovery;
mod probe;
mod stream;

use a4n_core::error::AppError;
use a4n_llm::provider::{LlmProvider, ModelCapabilities};
use a4n_service_protocol::{
    HealthStatus, LLM_SERVICE_NAME, ServiceHealth, known_service_health_for_host,
};
use axum::{Json, http::HeaderMap};

use crate::contracts::HttpLlmProviderConfig;

pub(crate) use discovery::discover_models;
pub(crate) use probe::probe;
#[cfg(test)]
pub(crate) use probe::probe_existing_provider;
#[cfg(test)]
pub(crate) use stream::llm_event_stream;
pub(crate) use stream::stream_chat;

pub(crate) async fn healthz(headers: HeaderMap) -> Json<ServiceHealth> {
    Json(known_service_health_for_host(
        LLM_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
        a4n_sidecar_bootstrap::host_protocol_version(&headers),
    ))
}

fn provider_from_config(config: HttpLlmProviderConfig) -> Result<Box<dyn LlmProvider>, AppError> {
    a4n_llm_http::http::pick_llm_provider(
        &config.provider,
        config.base_url,
        config.model,
        config.api_key,
        config.timeout_secs,
        config.extra_body_json,
        ModelCapabilities {
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
        },
    )
}
