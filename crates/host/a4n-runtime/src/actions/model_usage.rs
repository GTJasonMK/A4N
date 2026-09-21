use a4n_api_contract::cost::{ModelUsageReport, ModelUsageReportInput};
use a4n_error::AppError;

use crate::AppState;

pub async fn get_model_usage_report(
    state: &AppState,
    caller_user_id: i64,
    input: ModelUsageReportInput,
) -> Result<ModelUsageReport, AppError> {
    state
        .model_usage_read_application()
        .model_usage_report(caller_user_id, input)
        .await
}
