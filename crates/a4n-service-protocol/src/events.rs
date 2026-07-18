use serde::{Deserialize, Serialize};

use crate::artifacts::ArtifactRef;
use crate::response::ServiceError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ServiceTaskEvent {
    pub task_id: String,
    pub event_kind: ServiceTaskEventKind,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTaskEventKind {
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

impl ServiceTaskEventKind {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::CancelAck)
    }
}
