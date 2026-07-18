use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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
