#[derive(Default)]
pub(in crate::admin::generation_analytics) struct CallKindAccumulator {
    pub(in crate::admin::generation_analytics) calls: i64,
    pub(in crate::admin::generation_analytics) success: i64,
    pub(in crate::admin::generation_analytics) failed: i64,
    pub(in crate::admin::generation_analytics) pending: i64,
    pub(in crate::admin::generation_analytics) total_tokens: i64,
    pub(in crate::admin::generation_analytics) prompt_tokens: i64,
    pub(in crate::admin::generation_analytics) completion_tokens: i64,
    pub(in crate::admin::generation_analytics) prompt_cache_hit_tokens: i64,
    pub(in crate::admin::generation_analytics) prompt_cache_miss_tokens: i64,
    pub(in crate::admin::generation_analytics) duration_total_ms: i64,
    pub(in crate::admin::generation_analytics) duration_count: i64,
}

#[derive(Default)]
pub(in crate::admin::generation_analytics) struct LlmWorkflowAccumulator {
    pub(in crate::admin::generation_analytics) calls: i64,
    pub(in crate::admin::generation_analytics) success: i64,
    pub(in crate::admin::generation_analytics) failed: i64,
    pub(in crate::admin::generation_analytics) pending: i64,
    pub(in crate::admin::generation_analytics) total_tokens: i64,
    pub(in crate::admin::generation_analytics) prompt_tokens: i64,
    pub(in crate::admin::generation_analytics) completion_tokens: i64,
    pub(in crate::admin::generation_analytics) prompt_cache_hit_tokens: i64,
    pub(in crate::admin::generation_analytics) prompt_cache_miss_tokens: i64,
    pub(in crate::admin::generation_analytics) duration_total_ms: i64,
    pub(in crate::admin::generation_analytics) duration_count: i64,
    pub(in crate::admin::generation_analytics) stable_prefix_tokens_total: i64,
    pub(in crate::admin::generation_analytics) dynamic_delta_tokens_total: i64,
    pub(in crate::admin::generation_analytics) estimated_input_tokens_total: i64,
    pub(in crate::admin::generation_analytics) context_plan_count: i64,
    pub(in crate::admin::generation_analytics) dropped_optional_pack_count: i64,
    pub(in crate::admin::generation_analytics) context_over_budget_failure_count: i64,
    pub(in crate::admin::generation_analytics) estimated_actual_ratio_total: f64,
    pub(in crate::admin::generation_analytics) estimated_actual_ratio_count: i64,
}

#[derive(Default)]
pub(in crate::admin::generation_analytics) struct MediaWorkflowAccumulator {
    pub(in crate::admin::generation_analytics) calls: i64,
    pub(in crate::admin::generation_analytics) success: i64,
    pub(in crate::admin::generation_analytics) failed: i64,
    pub(in crate::admin::generation_analytics) pending: i64,
    pub(in crate::admin::generation_analytics) duration_total_ms: i64,
    pub(in crate::admin::generation_analytics) duration_count: i64,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::admin::generation_analytics) struct LlmContextPlanMetrics {
    pub(in crate::admin::generation_analytics) stable_prefix_tokens: i64,
    pub(in crate::admin::generation_analytics) dynamic_delta_tokens: i64,
    pub(in crate::admin::generation_analytics) estimated_input_tokens: i64,
    pub(in crate::admin::generation_analytics) dropped_optional_pack_count: i64,
}

pub(in crate::admin::generation_analytics) enum MediaStatusBucket {
    Success,
    Failed,
    Pending,
}
