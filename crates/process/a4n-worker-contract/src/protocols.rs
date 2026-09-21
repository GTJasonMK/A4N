use a4n_api_contract::agent::AgentScope;
use a4n_api_contract::chapter_visual_plan::{ChapterVisualPlanV3, VisualPlanReadView};
use a4n_api_contract::runtime_config::RuntimeConfigSnapshot;
use serde::{Deserialize, Serialize};

use crate::wire::{
    WorkerContractError, validate_positive_id, validate_positive_optional_id,
    validate_worker_schema_version,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerWaitRequest {
    pub run_id: i64,
    pub owner_user_id: i64,
    pub scope: AgentScope,
    pub execution_id: i64,
    pub tool_name: Option<String>,
    pub arguments_json: Option<serde_json::Value>,
    pub duration_exhausted: bool,
    pub runtime_config_snapshot: Option<RuntimeConfigSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerEditorProjectionRequest {
    pub plan: ChapterVisualPlanV3,
    pub view: VisualPlanReadView,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerCancelGenerationRequest {
    pub user_id: i64,
    pub task_id: String,
    pub attempt: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerCancelWorkflowRequest {
    pub workflow_run_id: i64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerLlmCapabilitiesRequest {
    pub schema_version: u32,
    pub user_id: i64,
    pub snapshot: Option<RuntimeConfigSnapshot>,
    pub config_id: Option<i64>,
}

impl AgentWorkerLlmCapabilitiesRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_worker_schema_version(self.schema_version)?;
        validate_positive_id("user_id", self.user_id)?;
        validate_positive_optional_id("config_id", self.config_id)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerChatRequest {
    pub schema_version: u32,
    pub user_id: i64,
    pub snapshot: Option<RuntimeConfigSnapshot>,
    pub config_id: Option<i64>,
    pub request: serde_json::Value,
}

impl AgentWorkerChatRequest {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_worker_schema_version(self.schema_version)?;
        validate_positive_id("user_id", self.user_id)?;
        if !self.request.is_object() {
            return Err(WorkerContractError::InvalidValue {
                field: "request",
                reason: "must be an object",
            });
        }
        validate_positive_optional_id("config_id", self.config_id)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentWorkerPackageExecutionResponse {
    pub enabled: bool,
}
