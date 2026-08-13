mod accumulators;

pub use a4n_domain::admin::{
    CallKindOverview, DailyCallMetric, GENERATION_ANALYTICS_MAX_DAYS,
    GENERATION_ANALYTICS_MAX_RECENT_LIMIT, GenerationCallAnalytics, GenerationCallAnalyticsInput,
    LlmWorkflowMetric, MediaWorkflowMetric, RecentLlmCall, RecentMediaCall,
};
pub(in crate::admin::generation_analytics) use accumulators::*;
