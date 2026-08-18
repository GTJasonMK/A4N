mod errors;
mod events;
mod health;
mod json;

#[cfg(test)]
pub(crate) use json::serialize_request;
pub use json::sidecar_response_payload;

use a4n_service_protocol::{
    SIDECAR_AUTH_HEADER, SIDECAR_REQUEST_ID_HEADER, SIDECAR_TRACE_ID_HEADER, ServiceError,
    ServiceErrorCode, ServiceRequest, expected_capability_version_for_service,
    redact_sensitive_message, sidecar_resource_policy,
};
use std::sync::Arc;
use std::time::Duration;

use crate::contracts::CapabilityResult;

pub type HttpClientFactory = Arc<dyn Fn() -> Result<reqwest::Client, String> + Send + Sync>;

#[cfg(test)]
pub(super) use events::{drain_event_lines, parse_event_line};
#[cfg(test)]
pub(super) use health::validate_service_health;

#[derive(Clone)]
pub(crate) struct HttpSidecarTransport {
    base_url: String,
    pub(super) client: reqwest::Client,
    connect_timeout: Duration,
    request_timeout: Duration,
    stream_idle_timeout: Duration,
    health_timeout: Duration,
    client_factory: Option<HttpClientFactory>,
    expected_service_name: &'static str,
    expected_capability_version: &'static str,
    auth_token: Option<String>,
}

impl std::fmt::Debug for HttpSidecarTransport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpSidecarTransport")
            .field("base_url", &"[REDACTED_URL]")
            .field("expected_service_name", &self.expected_service_name)
            .field(
                "expected_capability_version",
                &self.expected_capability_version,
            )
            .field("auth_token_configured", &self.auth_token.is_some())
            .finish()
    }
}

impl HttpSidecarTransport {
    pub(super) fn new(base_url: impl Into<String>, service_name: &'static str) -> Self {
        let capability_version = expected_capability_version_for_service(service_name)
            .unwrap_or_else(|| panic!("typed HTTP client service 未注册: {service_name}"));
        let policy = sidecar_resource_policy(service_name)
            .unwrap_or_else(|| panic!("typed HTTP client service 缺少资源策略: {service_name}"));
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(policy.connect_timeout)
                .build()
                .expect("static sidecar HTTP client configuration must be valid"),
            connect_timeout: policy.connect_timeout,
            request_timeout: policy.request_timeout,
            stream_idle_timeout: policy.stream_idle_timeout,
            health_timeout: policy.health_timeout,
            client_factory: None,
            expected_service_name: service_name,
            expected_capability_version: capability_version,
            auth_token: None,
        }
    }

    pub(super) fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    pub(super) fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.client_factory = Some(factory);
        self
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn with_test_timeouts(
        mut self,
        connect_timeout: Duration,
        request_timeout: Duration,
        stream_idle_timeout: Duration,
        health_timeout: Duration,
    ) -> Self {
        self.connect_timeout = connect_timeout;
        self.request_timeout = request_timeout;
        self.stream_idle_timeout = stream_idle_timeout;
        self.health_timeout = health_timeout;
        self
    }

    pub(super) fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub(super) fn authorize(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth_token {
            Some(token) => request.header(SIDECAR_AUTH_HEADER, token),
            None => request,
        }
    }

    pub(super) fn authorize_request<T>(
        &self,
        request: reqwest::RequestBuilder,
        envelope: &ServiceRequest<T>,
    ) -> CapabilityResult<reqwest::RequestBuilder> {
        let mut request = self.authorize(request);
        let request_id = reqwest::header::HeaderValue::from_str(&envelope.request_id).map_err(
            |error| {
                ServiceError::non_retryable(
                    ServiceErrorCode::ConfigurationError,
                    format!("request_id 不能作为 HTTP header: {error}"),
                )
            },
        )?;
        let trace_id =
            reqwest::header::HeaderValue::from_str(&envelope.trace_context.trace_id).map_err(
                |error| {
                    ServiceError::non_retryable(
                        ServiceErrorCode::ConfigurationError,
                        format!("trace_context.trace_id 不能作为 HTTP header: {error}"),
                    )
                },
            )?;
        request = request.header(SIDECAR_REQUEST_ID_HEADER, request_id);
        Ok(request.header(SIDECAR_TRACE_ID_HEADER, trace_id))
    }

    pub(super) fn request_client(&self) -> CapabilityResult<reqwest::Client> {
        self.client_factory
            .as_ref()
            .map(|factory| {
                factory().map_err(|error| {
                    ServiceError::new(
                        ServiceErrorCode::ServiceUnavailable,
                        redact_sensitive_message(error),
                        true,
                    )
                })
            })
            .unwrap_or_else(|| Ok(self.client.clone()))
    }
}
