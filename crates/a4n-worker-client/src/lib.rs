//! Minimal HTTP transport for stateful A4N workers.
//!
//! This crate intentionally does not depend on the capability sidecar
//! protocol. It transports the shared stateful-worker contracts and leaves
//! business DTOs to the worker-specific contract crates.

use std::time::Duration;

use reqwest::{Method, StatusCode, header::CONTENT_TYPE};
use serde::{Serialize, de::DeserializeOwned};

pub use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WorkerCancelRequest, WorkerCapabilities,
    WorkerCapability, WorkerContractError, WorkerEnvelope, WorkerError, WorkerErrorCode,
    WorkerHealth, WorkerHealthStatus, WorkerLease, WorkerOperationRef, WorkerOperationStatus,
    WorkerPagination, WorkerPrincipal, WorkerPrincipalKind, WorkerRecoveryRequest,
    WorkerResourceRef, WorkerResponse, WorkerRevision, WorkerScope,
};

pub const WORKER_AUTH_HEADER: &str = "x-a4n-worker-token";
pub const WORKER_COMMANDS_PATH: &str = "/v1/commands";
pub const WORKER_OPERATIONS_PATH: &str = "/v1/operations";

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub type WorkerJsonResponse = WorkerResponse<serde_json::Value>;

#[derive(Debug, Clone, Copy)]
pub struct WorkerClientConfig {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}

impl WorkerClientConfig {
    pub const fn new(connect_timeout: Duration, request_timeout: Duration) -> Self {
        Self {
            connect_timeout,
            request_timeout,
        }
    }
}

impl Default for WorkerClientConfig {
    fn default() -> Self {
        Self::new(DEFAULT_CONNECT_TIMEOUT, DEFAULT_REQUEST_TIMEOUT)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkerClientError {
    #[error("worker request contract is invalid: {0}")]
    InvalidRequest(#[source] WorkerContractError),
    #[error("worker response contract is invalid: {0}")]
    InvalidResponse(#[source] WorkerContractError),
    #[error("worker response identity does not match request: {field}")]
    ResponseIdentityMismatch { field: &'static str },
    #[error("worker client configuration is invalid: {0}")]
    Configuration(&'static str),
    #[error("worker request path segment is invalid: {field}")]
    InvalidPathSegment { field: &'static str },
    #[error("worker request timed out")]
    Timeout,
    #[error("worker request was unauthorized")]
    Unauthorized,
    #[error("worker returned HTTP status {status}")]
    HttpStatus { status: u16 },
    #[error("worker HTTP transport failed")]
    Transport(#[source] reqwest::Error),
    #[error("worker request JSON serialization failed")]
    RequestJson(#[source] serde_json::Error),
    #[error("worker response JSON is invalid")]
    ResponseJson(#[source] serde_json::Error),
}

#[derive(Clone)]
pub struct HttpWorkerClient {
    base_url: String,
    client: reqwest::Client,
    expected_worker_id: String,
    request_timeout: Duration,
    auth_token: Option<String>,
}

impl std::fmt::Debug for HttpWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpWorkerClient")
            .field("base_url", &"[REDACTED_URL]")
            .field("expected_worker_id", &self.expected_worker_id)
            .field("auth_token_configured", &self.auth_token.is_some())
            .finish()
    }
}

impl HttpWorkerClient {
    pub fn new(base_url: impl Into<String>, expected_worker_id: impl Into<String>) -> Self {
        Self::with_config(base_url, expected_worker_id, WorkerClientConfig::default())
    }

    pub fn with_config(
        base_url: impl Into<String>,
        expected_worker_id: impl Into<String>,
        config: WorkerClientConfig,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .connect_timeout(config.connect_timeout)
                .build()
                .expect("static worker HTTP client configuration must be valid"),
            expected_worker_id: expected_worker_id.into(),
            request_timeout: config.request_timeout,
            auth_token: None,
        }
    }

    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    pub async fn health(&self) -> Result<WorkerHealth, WorkerClientError> {
        self.validate_expected_worker_id()?;
        let health = self
            .request_json::<(), WorkerHealth>(Method::GET, WORKER_HEALTH_PATH, None)
            .await?;
        health
            .validate_for(&self.expected_worker_id)
            .map_err(WorkerClientError::InvalidResponse)?;
        Ok(health)
    }

    pub async fn capabilities(&self) -> Result<WorkerCapabilities, WorkerClientError> {
        self.validate_expected_worker_id()?;
        let capabilities = self
            .request_json::<(), WorkerCapabilities>(Method::GET, WORKER_CAPABILITIES_PATH, None)
            .await?;
        capabilities
            .validate_for(&self.expected_worker_id)
            .map_err(WorkerClientError::InvalidResponse)?;
        Ok(capabilities)
    }

    pub async fn command<I, O>(
        &self,
        command: &str,
        request: WorkerEnvelope<I>,
    ) -> Result<WorkerResponse<O>, WorkerClientError>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        request
            .validate()
            .map_err(WorkerClientError::InvalidRequest)?;
        let path = command_path(command)?;
        let response = self
            .request_json(Method::POST, &path, Some(&request))
            .await?;
        validate_command_response(&request, &response)?;
        Ok(response)
    }

    pub async fn cancel(
        &self,
        request: WorkerCancelRequest,
    ) -> Result<WorkerJsonResponse, WorkerClientError> {
        request
            .validate()
            .map_err(WorkerClientError::InvalidRequest)?;
        let path = operation_path(&request.operation.operation_id, "cancel")?;
        let response = self
            .request_json(Method::POST, &path, Some(&request))
            .await?;
        validate_response(&response)?;
        Ok(response)
    }

    pub async fn recover(
        &self,
        request: WorkerRecoveryRequest,
    ) -> Result<WorkerJsonResponse, WorkerClientError> {
        request
            .validate()
            .map_err(WorkerClientError::InvalidRequest)?;
        let path = operation_path(&request.operation.operation_id, "recover")?;
        let response = self
            .request_json(Method::POST, &path, Some(&request))
            .await?;
        validate_response(&response)?;
        Ok(response)
    }

    async fn request_json<B, O>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<O, WorkerClientError>
    where
        B: Serialize,
        O: DeserializeOwned,
    {
        if self.base_url.is_empty() {
            return Err(WorkerClientError::Configuration(
                "base_url must not be blank",
            ));
        }
        let request = self.authorize(self.client.request(method, self.url(path)));
        let request = match body {
            Some(body) => request
                .header(CONTENT_TYPE, "application/json")
                .body(serde_json::to_vec(body).map_err(WorkerClientError::RequestJson)?),
            None => request,
        };
        let (status, body) = tokio::time::timeout(self.request_timeout, async {
            let response = request.send().await.map_err(map_transport_error)?;
            let status = response.status();
            let body = response.bytes().await.map_err(map_transport_error)?;
            Ok::<_, WorkerClientError>((status, body))
        })
        .await
        .map_err(|_| WorkerClientError::Timeout)??;

        if status == StatusCode::UNAUTHORIZED {
            return Err(WorkerClientError::Unauthorized);
        }
        if !status.is_success() {
            return Err(WorkerClientError::HttpStatus {
                status: status.as_u16(),
            });
        }

        serde_json::from_slice(&body).map_err(WorkerClientError::ResponseJson)
    }

    fn authorize(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth_token {
            Some(token) => request.header(WORKER_AUTH_HEADER, token),
            None => request,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn validate_expected_worker_id(&self) -> Result<(), WorkerClientError> {
        if self.expected_worker_id.trim().is_empty() {
            return Err(WorkerClientError::Configuration(
                "expected_worker_id must not be blank",
            ));
        }
        Ok(())
    }
}

fn validate_response<T>(response: &WorkerResponse<T>) -> Result<(), WorkerClientError> {
    response
        .validate()
        .map_err(WorkerClientError::InvalidResponse)
}

fn validate_command_response<I, O>(
    request: &WorkerEnvelope<I>,
    response: &WorkerResponse<O>,
) -> Result<(), WorkerClientError> {
    validate_response(response)?;
    if response.request_id != request.request_id {
        return Err(WorkerClientError::ResponseIdentityMismatch {
            field: "request_id",
        });
    }
    if response.trace_id != request.trace_id {
        return Err(WorkerClientError::ResponseIdentityMismatch { field: "trace_id" });
    }
    Ok(())
}

fn command_path(command: &str) -> Result<String, WorkerClientError> {
    validate_path_segment("command", command)?;
    Ok(format!("{WORKER_COMMANDS_PATH}/{command}"))
}

fn operation_path(operation_id: &str, action: &str) -> Result<String, WorkerClientError> {
    validate_path_segment("operation_id", operation_id)?;
    Ok(format!("{WORKER_OPERATIONS_PATH}/{operation_id}/{action}"))
}

fn validate_path_segment(field: &'static str, value: &str) -> Result<(), WorkerClientError> {
    if value.is_empty()
        || matches!(value, "." | "..")
        || value.chars().any(|character| {
            !character.is_ascii()
                || character.is_ascii_control()
                || character.is_whitespace()
                || matches!(character, '/' | '\\' | '?' | '#' | '%')
        })
    {
        return Err(WorkerClientError::InvalidPathSegment { field });
    }
    Ok(())
}

fn map_transport_error(error: reqwest::Error) -> WorkerClientError {
    if error.is_timeout() {
        WorkerClientError::Timeout
    } else {
        WorkerClientError::Transport(error.without_url())
    }
}

#[cfg(test)]
mod tests;
