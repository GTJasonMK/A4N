use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
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
    pub prompt_cache_hit_ratio: Option<f64>,
    pub avg_duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DailyCallMetric {
    pub date: String,
    pub llm_calls: i64,
    pub image_calls: i64,
    pub video_calls: i64,
    pub llm_total_tokens: i64,
    pub llm_prompt_cache_hit_tokens: i64,
    pub llm_prompt_cache_miss_tokens: i64,
}

#[derive(Debug, Clone, Serialize)]
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
    pub prompt_cache_hit_ratio: Option<f64>,
    pub avg_duration_ms: Option<i64>,
    pub avg_stable_prefix_tokens: Option<i64>,
    pub avg_dynamic_delta_tokens: Option<i64>,
    pub avg_estimated_input_tokens: Option<i64>,
    pub dropped_optional_pack_count: i64,
    pub context_over_budget_failure_count: i64,
    pub estimated_vs_actual_prompt_tokens_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
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
