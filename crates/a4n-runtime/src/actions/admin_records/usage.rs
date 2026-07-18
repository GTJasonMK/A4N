use super::{AdminOverviewSummary, DailyUsagePoint, UsageLogSummary, UsageTrendsInput};
use crate::{AppState, actions::app_error_message};
use serde_json::Value;

pub async fn get_user_usage(
    state: &AppState,
    user_id: i64,
    limit: u64,
) -> Result<Vec<UsageLogSummary>, String> {
    state
        .orchestrator
        .user_usage(user_id, limit)
        .await
        .map_err(app_error_message)
}

pub async fn get_token_summary(state: &AppState, user_id: i64) -> Result<Value, String> {
    state
        .orchestrator
        .token_summary(user_id)
        .await
        .map_err(app_error_message)
}

pub async fn get_usage_trends(
    state: &AppState,
    input: UsageTrendsInput,
) -> Result<Vec<DailyUsagePoint>, String> {
    state
        .orchestrator
        .admin_usage_trends(input)
        .await
        .map_err(app_error_message)
}

pub async fn get_admin_overview(state: &AppState) -> Result<AdminOverviewSummary, String> {
    state
        .orchestrator
        .admin_overview()
        .await
        .map_err(app_error_message)
}
