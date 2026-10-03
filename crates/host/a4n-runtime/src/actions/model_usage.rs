use a4n_api_contract::cost::{
    ModelUsageReport, ModelUsageReportInput, ModelUsageRequest, ModelUsageRequestInput,
};
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

pub async fn get_model_usage_request(
    state: &AppState,
    caller_user_id: i64,
    input: ModelUsageRequestInput,
) -> Result<ModelUsageRequest, AppError> {
    state
        .model_usage_read_application()
        .model_usage_request(caller_user_id, input)
        .await
}
