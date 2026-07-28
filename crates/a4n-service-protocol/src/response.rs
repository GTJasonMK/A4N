use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::artifacts::ArtifactRef;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceResponse<T> {
    pub status: ServiceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<ArtifactRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ServiceError>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceError {
    pub code: ServiceErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ServiceErrorCode {
    #[error("capability disabled")]
    CapabilityDisabled,
    #[error("service unavailable")]
    ServiceUnavailable,
    #[error("version mismatch")]
    VersionMismatch,
    #[error("configuration error")]
    ConfigurationError,
    #[error("unauthorized")]
    Unauthorized,
    #[error("cancelled")]
    Cancelled,
    #[error("provider error")]
    ProviderError,
    #[error("internal error")]
    Internal,
}
