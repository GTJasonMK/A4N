mod chat;
mod discovery;
mod probe;
mod stream;

use a4n_core::error::AppError;
use a4n_llm_http::http::HttpLlmProvider;
use a4n_service_protocol::{
    HealthStatus, LLM_CHAT_CAPABILITY_VERSION as LLM_CAPABILITY_VERSION, LLM_SERVICE_NAME,
    PROTOCOL_VERSION, ServiceHealth, ServiceVersion,
};
use axum::Json;

use crate::contracts::HttpLlmProviderConfig;

pub(crate) use chat::chat;
pub(crate) use discovery::discover_models;
pub(crate) use probe::probe;
#[cfg(test)]
pub(crate) use probe::probe_existing_provider;
#[cfg(test)]
pub(crate) use stream::llm_event_stream;
pub(crate) use stream::stream_chat;

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: LLM_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: LLM_CAPABILITY_VERSION.to_string(),
        },
    })
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
