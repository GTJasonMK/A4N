use super::{
    metrics::{CallKindOverview, DailyCallMetric, LlmWorkflowMetric, MediaWorkflowMetric},
    recent::{RecentLlmCall, RecentMediaCall},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationCallAnalyticsInput {
    pub days: Option<u32>,
    pub recent_limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationCallAnalytics {
    pub generated_at: String,
    pub since_at: String,
    pub days: u32,
    pub overview: Vec<CallKindOverview>,
    pub daily: Vec<DailyCallMetric>,
    pub llm_by_workflow: Vec<LlmWorkflowMetric>,
    pub media_by_workflow: Vec<MediaWorkflowMetric>,
    pub recent_llm_calls: Vec<RecentLlmCall>,
    pub recent_media_calls: Vec<RecentMediaCall>,
}
