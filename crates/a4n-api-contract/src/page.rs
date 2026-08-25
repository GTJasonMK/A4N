use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::command_manifest::CommandDescriptor;

pub const PAGE_SESSION_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PageSessionStatus {
    Idle,
    AwaitingAttach,
    Attached,
    Running,
    Paused,
    Offline,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PageOperationStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PageContext {
    pub page_kind: Option<String>,
    pub route: Option<String>,
    pub project_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub active_tab: Option<String>,
    pub selected_ids: Value,
    pub dirty_state: bool,
    pub local_state: Value,
    pub context_revision: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct PageSessionView {
    pub protocol_version: u32,
    pub ui_session_id: String,
    pub mode: String,
    pub status: PageSessionStatus,
    pub client: String,
    pub tab_id: String,
    pub page_kind: Option<String>,
    pub route: Option<String>,
    pub project_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub active_tab: Option<String>,
    pub selected_ids: Value,
    pub dirty_state: bool,
    pub local_state: Value,
    pub context_revision: u64,
    pub agent_id: Option<String>,
    pub operation_id: Option<String>,
    pub last_operation: Option<PageOperationView>,
    pub created_at: String,
    pub updated_at: String,
    pub last_seen_at: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct PageOperationView {
    pub operation_id: String,
    pub ui_session_id: String,
    pub agent_id: Option<String>,
    pub command: String,
    pub page_kind: Option<String>,
    pub phase: String,
    pub target: Value,
    pub status: PageOperationStatus,
    pub progress: Option<f32>,
    pub summary: Option<String>,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub started_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct PageSessionEvent {
    pub kind: String,
    pub session: PageSessionView,
    pub operation: Option<PageOperationView>,
    pub route: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PageAuthorization {
    pub ui_session_id: String,
    pub authorization_code: String,
    pub expires_in: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct PageSessionCreateResponse {
    pub session: PageSessionView,
    pub ui_session_token: String,
    pub lease_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct PageAttachResponse {
    pub session: PageSessionView,
    pub ui_session_token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PageOperationInput {
    pub command: String,
    #[serde(default)]
    pub allow_destructive: bool,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub target: Value,
    #[serde(default)]
    pub context_revision: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PageOperationUpdate {
    #[serde(default)]
    pub status: Option<PageOperationStatus>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub progress: Option<f32>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PageNavigationInput {
    pub route: String,
    #[serde(default)]
    pub page_kind: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct PageCommandAvailability {
    pub descriptor: CommandDescriptor,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct PageCommandManifest {
    pub protocol_version: u32,
    pub ui_session_id: String,
    pub page_kind: Option<String>,
    pub route: Option<String>,
    pub commands: Vec<PageCommandAvailability>,
}
