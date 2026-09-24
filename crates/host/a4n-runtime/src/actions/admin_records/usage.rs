use super::{AdminOverviewSummary, DailyUsagePoint, UsageLogSummary, UsageTrendsInput};
use crate::AppState;
use serde_json::Value;

pub(super) async fn get_user_usage(
    state: &AppState,
    caller_user_id: i64,
    user_id: i64,
    limit: u64,
) -> Result<Vec<UsageLogSummary>, String> {
    state
        .admin_read_application()
        .user_usage(caller_user_id, user_id, limit)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn get_token_summary(
    state: &AppState,
    caller_user_id: i64,
    user_id: i64,
) -> Result<Value, String> {
    state
        .admin_read_application()
        .token_summary(caller_user_id, user_id)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn get_usage_trends(
    state: &AppState,
    caller_user_id: i64,
    input: UsageTrendsInput,
) -> Result<Vec<DailyUsagePoint>, String> {
    state
        .admin_read_application()
        .usage_trends(caller_user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn get_admin_overview(
    state: &AppState,
    caller_user_id: i64,
) -> Result<AdminOverviewSummary, String> {
    state
        .admin_read_application()
        .overview(caller_user_id)
        .await
        .map_err(|error| error.to_string())
}
