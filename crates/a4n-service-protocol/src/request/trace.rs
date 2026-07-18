use crate::registry::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceContext {
    pub trace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SchemaVersion {
    pub protocol: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
}

impl SchemaVersion {
    pub fn protocol_only() -> Self {
        Self {
            protocol: PROTOCOL_VERSION,
            database: None,
        }
    }

    pub fn is_protocol_only(&self) -> bool {
        self.database.is_none()
    }
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self::protocol_only()
    }
}
