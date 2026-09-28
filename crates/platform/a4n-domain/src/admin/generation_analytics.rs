use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const GENERATION_ANALYTICS_MAX_DAYS: u32 = 180;
pub const GENERATION_ANALYTICS_MAX_RECENT_LIMIT: u64 = 200;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationCallAnalyticsInput {
    #[schemars(range(min = 1, max = 180))]
    pub days: u32,
    #[schemars(range(min = 1, max = 200))]
    pub recent_limit: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CallKindOverview {
    pub kind: String,
    pub calls: i64,
    pub success: i64,
    pub failed: i64,
    pub pending: i64,
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub prompt_cache_hit_tokens: i64,
    pub prompt_cache_miss_tokens: i64,
    pub prompt_cache_write_tokens: Option<i64>,
    pub prompt_cache_hit_ratio: Option<f64>,
    pub avg_duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DailyCallMetric {
    pub date: String,
    pub llm_calls: i64,
    pub image_calls: i64,
    pub video_calls: i64,
    pub llm_total_tokens: i64,
    pub llm_prompt_cache_hit_tokens: i64,
    pub llm_prompt_cache_miss_tokens: i64,
    pub llm_prompt_cache_write_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct LlmWorkflowMetric {
    pub workflow: String,
    pub step_key: String,
    pub calls: i64,
    pub success: i64,
    pub failed: i64,
    pub pending: i64,
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub prompt_cache_hit_tokens: i64,
    pub prompt_cache_miss_tokens: i64,
    pub prompt_cache_write_tokens: Option<i64>,
    pub prompt_cache_hit_ratio: Option<f64>,
    pub avg_duration_ms: Option<i64>,
    pub avg_stable_prefix_tokens: Option<i64>,
    pub avg_dynamic_delta_tokens: Option<i64>,
    pub avg_estimated_input_tokens: Option<i64>,
    pub dropped_optional_pack_count: i64,
    pub context_over_budget_failure_count: i64,
    pub estimated_vs_actual_prompt_tokens_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MediaWorkflowMetric {
    pub kind: String,
    pub workflow: String,
    pub step_key: String,
    pub calls: i64,
    pub success: i64,
    pub failed: i64,
    pub pending: i64,
    pub avg_duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RecentLlmCall {
    pub id: i64,
    pub created_at: String,
    pub workflow: String,
    pub step_key: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub prompt_cache_hit_tokens: Option<i32>,
    pub prompt_cache_miss_tokens: Option<i32>,
    pub prompt_cache_write_tokens: Option<i32>,
    pub prompt_cache_hit_ratio: Option<f64>,
    pub duration_ms: Option<i64>,
    pub streamed: bool,
    pub status: String,
    pub error_kind: Option<String>,
    pub finish_reason: Option<String>,
    pub stable_prefix_tokens: Option<i64>,
    pub dynamic_delta_tokens: Option<i64>,
    pub estimated_input_tokens: Option<i64>,
    pub dropped_optional_pack_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RecentMediaCall {
    pub id: i64,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub kind: String,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub duration_ms: Option<i64>,
}
