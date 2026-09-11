use super::{AdminOverviewSummary, DailyUsagePoint, UsageLogSummary, UsageTrendsInput};
use crate::AppState;
use serde_json::Value;

pub(super) async fn get_user_usage(
    state: &AppState,
    user_id: i64,
    limit: u64,
) -> Result<Vec<UsageLogSummary>, String> {
    state
        .admin_application()
        .user_usage(user_id, limit)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn get_token_summary(state: &AppState, user_id: i64) -> Result<Value, String> {
    state
        .admin_application()
        .token_summary(user_id)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn get_usage_trends(
    state: &AppState,
    input: UsageTrendsInput,
) -> Result<Vec<DailyUsagePoint>, String> {
    state
        .admin_application()
        .usage_trends(input)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn get_admin_overview(state: &AppState) -> Result<AdminOverviewSummary, String> {
    state
        .admin_application()
        .overview()
        .await
        .map_err(|error| error.to_string())
}
