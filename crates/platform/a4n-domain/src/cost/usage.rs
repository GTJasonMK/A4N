use super::contracts::{ModelCapability, ModelUsageMeter, PricingPlan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageRecordStart {
    pub request_id: String,
    pub user_id: Option<i64>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub generation_run_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub call_key: Option<String>,
    pub source: Option<String>,
    pub capability: ModelCapability,
    pub provider_config_id: Option<i64>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub pricing: PricingPlan,
    pub request_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageRecordCompletion {
    pub status: String,
    pub meter: ModelUsageMeter,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
}
