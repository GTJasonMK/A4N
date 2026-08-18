use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::artifacts::ArtifactRef;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    deny_unknown_fields,
    bound(deserialize = "T: Deserialize<'de>")
)]
pub struct ServiceResponse<T> {
    pub status: ServiceStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<ArtifactRef>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present_payload"
    )]
    pub payload: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ServiceError>,
}

fn deserialize_present_payload<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

impl<T> ServiceResponse<T> {
    pub fn ok(trace_id: String, payload: T) -> Self {
        Self {
            status: ServiceStatus::Ok,
            trace_id: Some(trace_id),
            usage: None,
            artifact_refs: Vec::new(),
            payload: Some(payload),
            error: None,
        }
    }

    pub fn error(trace_id: String, error: ServiceError) -> Self {
        Self {
            status: ServiceStatus::Error,
            trace_id: Some(trace_id),
            usage: None,
            artifact_refs: Vec::new(),
            payload: None,
            error: Some(error),
        }
    }

    pub fn from_result<E>(
        trace_id: String,
        result: Result<T, E>,
        map_error: impl FnOnce(E) -> ServiceError,
    ) -> Self {
        match result {
            Ok(payload) => Self::ok(trace_id, payload),
            Err(error) => Self::error(trace_id, map_error(error)),
        }
    }
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

impl ServiceError {
    pub fn new(code: ServiceErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: Some(retryable),
        }
    }

    pub fn non_retryable(code: ServiceErrorCode, message: impl Into<String>) -> Self {
        Self::new(code, message, false)
    }
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
