mod discovery;
mod probe;
mod stream;

use a4n_core::error::AppError;
use a4n_llm_http::http::HttpLlmProvider;
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

fn provider_from_config(config: HttpLlmProviderConfig) -> Result<HttpLlmProvider, AppError> {
    HttpLlmProvider::new_with_timeout_and_options(
        config.base_url,
        config.model,
        config.api_key,
        config.timeout_secs,
        config.request_options_json,
    )
}
