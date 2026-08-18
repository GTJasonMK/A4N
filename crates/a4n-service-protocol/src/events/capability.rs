use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::response::{ServiceError, ServiceErrorCode};
use crate::sanitization::redact_sensitive_message;

/// Sidecar capability 传输事件；它承载 provider 原始响应，不是 Host generation 领域事件。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityTaskEvent {
    pub task_id: String,
    pub event_kind: CapabilityTaskEventKind,
    pub sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ServiceError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl CapabilityTaskEvent {
    fn empty(task_id: String, event_kind: CapabilityTaskEventKind, sequence: u64) -> Self {
        Self {
            task_id,
            event_kind,
            sequence,
            error: None,
            payload: None,
        }
    }

    fn serialized_payload(
        payload: impl Serialize,
        serialization_context: &str,
    ) -> Result<serde_json::Value, ServiceError> {
        serde_json::to_value(payload)
            .map_err(|error| {
                ServiceError::non_retryable(
                    ServiceErrorCode::Internal,
                    redact_sensitive_message(format!("{serialization_context}: {error}")),
                )
            })
    }

    pub fn progress(task_id: String, sequence: u64) -> Self {
        Self::empty(task_id, CapabilityTaskEventKind::Progress, sequence)
    }

    pub fn progress_with_payload(
        task_id: String,
        sequence: u64,
        payload: impl Serialize,
        serialization_context: &str,
    ) -> Result<Self, ServiceError> {
        Self::serialized_payload(payload, serialization_context).map(|payload| Self {
            payload: Some(payload),
            ..Self::progress(task_id, sequence)
        })
    }

    pub fn progress_message(
        task_id: String,
        sequence: u64,
        message: impl Into<String>,
    ) -> Self {
        Self {
            payload: Some(serde_json::json!({ "message": message.into() })),
            ..Self::progress(task_id, sequence)
        }
    }

    pub fn chunk(
        task_id: String,
        sequence: u64,
        payload: impl Serialize,
        serialization_context: &str,
    ) -> Result<Self, ServiceError> {
        Self::serialized_payload(payload, serialization_context).map(|payload| Self {
            payload: Some(payload),
            ..Self::empty(task_id, CapabilityTaskEventKind::Chunk, sequence)
        })
    }

    pub fn completed_or_failed(
        task_id: String,
        sequence: u64,
        payload: impl Serialize,
        serialization_context: &str,
    ) -> Self {
        match Self::serialized_payload(payload, serialization_context) {
            Ok(payload) => Self::completed_value(task_id, sequence, payload),
            Err(error) => Self::failed(task_id, sequence, error),
        }
    }

    fn completed_value(
        task_id: String,
        sequence: u64,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            payload: Some(payload),
            ..Self::completed_empty(task_id, sequence)
        }
    }

    pub fn completed_empty(task_id: String, sequence: u64) -> Self {
        Self::empty(task_id, CapabilityTaskEventKind::Completed, sequence)
    }

    pub fn failed(task_id: String, sequence: u64, error: ServiceError) -> Self {
        let mut event = Self::empty(task_id, CapabilityTaskEventKind::Failed, sequence);
        event.error = Some(error);
        event
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityTaskEventKind {
    Progress,
    Chunk,
    Completed,
    Failed,
}

impl CapabilityTaskEventKind {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}
