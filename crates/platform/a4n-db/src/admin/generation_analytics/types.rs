pub use a4n_domain::admin::{
    CallKindOverview, DailyCallMetric, GENERATION_ANALYTICS_MAX_DAYS,
    GENERATION_ANALYTICS_MAX_RECENT_LIMIT, GenerationCallAnalytics, GenerationCallAnalyticsInput,
    LlmWorkflowMetric, MediaWorkflowMetric, RecentLlmCall, RecentMediaCall,
};

#[derive(Default)]
pub(in crate::admin::generation_analytics) struct CallKindAccumulator {
    pub(super) cache_usage_incomplete: bool,
    pub(super) calls: i64,
    pub(super) success: i64,
    pub(super) failed: i64,
    pub(super) pending: i64,
    pub(super) total_tokens: i64,
    pub(super) prompt_tokens: i64,
    pub(super) completion_tokens: i64,
    pub(super) prompt_cache_hit_tokens: i64,
    pub(super) prompt_cache_miss_tokens: i64,
    pub(super) prompt_cache_write_tokens: Option<i64>,
    pub(super) duration_total_ms: i64,
    pub(super) duration_count: i64,
}

#[derive(Default)]
pub(in crate::admin::generation_analytics) struct LlmWorkflowAccumulator {
    pub(super) cache_usage_incomplete: bool,
    pub(super) calls: i64,
    pub(super) success: i64,
    pub(super) failed: i64,
    pub(super) pending: i64,
    pub(super) total_tokens: i64,
    pub(super) prompt_tokens: i64,
    pub(super) completion_tokens: i64,
    pub(super) prompt_cache_hit_tokens: i64,
    pub(super) prompt_cache_miss_tokens: i64,
    pub(super) prompt_cache_write_tokens: Option<i64>,
    pub(super) duration_total_ms: i64,
    pub(super) duration_count: i64,
    pub(super) stable_prefix_tokens_total: i64,
    pub(super) dynamic_delta_tokens_total: i64,
    pub(super) estimated_input_tokens_total: i64,
    pub(super) context_plan_count: i64,
    pub(super) dropped_optional_pack_count: i64,
    pub(super) context_over_budget_failure_count: i64,
    pub(super) estimated_actual_ratio_total: f64,
    pub(super) estimated_actual_ratio_count: i64,
}

#[derive(Default)]
pub(in crate::admin::generation_analytics) struct MediaWorkflowAccumulator {
    pub(super) calls: i64,
    pub(super) success: i64,
    pub(super) failed: i64,
    pub(super) pending: i64,
    pub(super) duration_total_ms: i64,
    pub(super) duration_count: i64,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::admin::generation_analytics) struct LlmContextPlanMetrics {
    pub(super) stable_prefix_tokens: i64,
    pub(super) dynamic_delta_tokens: i64,
    pub(super) estimated_input_tokens: i64,
    pub(super) dropped_optional_pack_count: i64,
}

pub(in crate::admin::generation_analytics) enum MediaStatusBucket {
    Success,
    Failed,
    Pending,
}
