mod errors;
mod events;
mod health;
mod json;

use a4n_service_protocol::{
    SIDECAR_AUTH_HEADER, SIDECAR_CONNECT_TIMEOUT, SIDECAR_HEALTH_TIMEOUT,
    SIDECAR_REQUEST_ID_HEADER, SIDECAR_REQUEST_TIMEOUT, SIDECAR_STREAM_IDLE_TIMEOUT,
    SIDECAR_TRACE_ID_HEADER, ServiceError, ServiceErrorCode,
    expected_capability_version_for_service, redact_sensitive_message,
};
use std::sync::Arc;
use std::time::Duration;

use crate::contracts::{CapabilityResult, JsonRequest};

pub type HttpClientFactory = Arc<dyn Fn() -> Result<reqwest::Client, String> + Send + Sync>;

#[cfg(test)]
pub(super) use events::{drain_event_lines, parse_event_line};
#[cfg(test)]
pub(super) use health::{validate_health_expectation, validate_service_health};

#[derive(Clone)]
pub struct HttpSidecarTransport {
    base_url: String,
    pub(super) client: reqwest::Client,
    pub(super) connect_timeout: Duration,
    pub(super) request_timeout: Duration,
    pub(super) stream_idle_timeout: Duration,
    pub(super) health_timeout: Duration,
    pub(super) client_factory: Option<HttpClientFactory>,
    pub(super) expected_service_name: Option<&'static str>,
    pub(super) expected_capability_version: Option<&'static str>,
    pub(super) auth_token: Option<String>,
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
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(SIDECAR_CONNECT_TIMEOUT)
                .build()
                .expect("static sidecar HTTP client configuration must be valid"),
            connect_timeout: SIDECAR_CONNECT_TIMEOUT,
            request_timeout: SIDECAR_REQUEST_TIMEOUT,
            stream_idle_timeout: SIDECAR_STREAM_IDLE_TIMEOUT,
            health_timeout: SIDECAR_HEALTH_TIMEOUT,
            client_factory: None,
            expected_service_name: None,
            expected_capability_version: None,
            auth_token: None,
        }
    }

    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self.client_factory = None;
        self
    }

    pub fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.client_factory = Some(factory);
        self
    }

    pub(super) fn with_expected_service(mut self, service_name: &'static str) -> Self {
        self.expected_service_name = Some(service_name);
        self.expected_capability_version = expected_capability_version_for_service(service_name);
        if let Some(policy) = a4n_service_protocol::sidecar_resource_policy(service_name) {
            self.connect_timeout = policy.connect_timeout;
            self.request_timeout = policy.request_timeout;
            self.stream_idle_timeout = policy.stream_idle_timeout;
            self.health_timeout = policy.health_timeout;
        }
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

    pub(super) fn authorize_request(
        &self,
        request: reqwest::RequestBuilder,
        envelope: &JsonRequest,
    ) -> reqwest::RequestBuilder {
        let mut request = self.authorize(request);
        if let Ok(value) = reqwest::header::HeaderValue::from_str(&envelope.request_id) {
            request = request.header(SIDECAR_REQUEST_ID_HEADER, value);
        }
        if let Ok(value) = reqwest::header::HeaderValue::from_str(&envelope.trace_context.trace_id)
        {
            request = request.header(SIDECAR_TRACE_ID_HEADER, value);
        }
        request
    }

    pub(super) fn request_client(&self) -> CapabilityResult<reqwest::Client> {
        self.client_factory
            .as_ref()
            .map(|factory| {
                factory().map_err(|error| ServiceError {
                    code: ServiceErrorCode::ServiceUnavailable,
                    message: redact_sensitive_message(error),
                    retryable: Some(true),
                })
            })
            .unwrap_or_else(|| Ok(self.client.clone()))
    }
}
