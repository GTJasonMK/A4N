use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ModelUsageReportInput {
    pub project_id: Option<i64>,
    pub days: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsageCurrencyTotal {
    pub currency: String,
    pub amount_micros: String,
    pub priced_calls: u64,
    pub partial_calls: u64,
    pub unpriced_calls: u64,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsageCapabilityTotal {
    pub currency: String,
    pub capability: String,
    pub calls: u64,
    pub amount_micros: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsageEntry {
    pub id: i64,
    pub request_id: String,
    pub capability: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub currency: String,
    pub amount_micros: Option<String>,
    pub confidence: String,
    pub status: String,
    pub meter: serde_json::Value,
    pub source: Option<String>,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsageReport {
    pub project_id: Option<i64>,
    pub since: String,
    pub until: String,
    pub totals: Vec<ModelUsageCurrencyTotal>,
    pub capabilities: Vec<ModelUsageCapabilityTotal>,
    pub recent: Vec<ModelUsageEntry>,
    pub partial_calls: u64,
    pub unpriced_calls: u64,
    pub unsettled_calls: u64,
}

#[async_trait::async_trait]
pub trait ModelUsageReadApplicationPort: Send + Sync {
    async fn model_usage_report(
        &self,
        caller_user_id: i64,
        input: ModelUsageReportInput,
    ) -> Result<ModelUsageReport, a4n_error::AppError>;
}
