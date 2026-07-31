use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::artifacts::ArtifactRef;
use crate::response::{ServiceError, ServiceErrorCode};
use crate::sanitization::redact_sensitive_message;

/// Sidecar capability 传输事件；它承载 provider 原始响应，不是 Host generation 领域事件。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CapabilityTaskEvent {
    pub task_id: String,
    pub event_kind: CapabilityTaskEventKind,
    pub sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_delta: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_usage: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_truncated_finish_reason: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<ArtifactRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ServiceError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl CapabilityTaskEvent {
    pub fn new(task_id: String, event_kind: CapabilityTaskEventKind, sequence: u64) -> Self {
        Self {
            task_id,
            event_kind,
            sequence,
            chunk_index: None,
            chunk_delta: None,
            usage_delta: None,
            final_usage: None,
            finish_reason: None,
            is_truncated_finish_reason: None,
            checkpoint_key: None,
            artifact_refs: Vec::new(),
            error: None,
            payload: None,
        }
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn with_serialized_payload(
        self,
        payload: impl Serialize,
        serialization_context: &str,
    ) -> Result<Self, ServiceError> {
        serde_json::to_value(payload)
            .map(|payload| self.with_payload(payload))
            .map_err(|error| {
                ServiceError::non_retryable(
                    ServiceErrorCode::Internal,
                    redact_sensitive_message(format!("{serialization_context}: {error}")),
                )
            })
    }

    pub fn failed(task_id: String, sequence: u64, error: ServiceError) -> Self {
        let mut event = Self::new(task_id, CapabilityTaskEventKind::Failed, sequence);
        event.error = Some(error);
        event
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityTaskEventKind {
    Progress,
    Chunk,
    StructuredProgress,
    Usage,
    Checkpoint,
    Artifact,
    Completed,
    Failed,
    CancelAck,
}

impl CapabilityTaskEventKind {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::CancelAck)
    }
}
