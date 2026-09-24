use crate::{
    AppState,
    actions::admin_records::{AdminPermission, require_admin_permission},
};
pub use a4n_api_contract::admin::{GenerationCallAnalytics, GenerationCallAnalyticsInput};
use a4n_error::AppError;

pub async fn get_generation_call_analytics_for_admin(
    state: &AppState,
    caller_user_id: i64,
    input: GenerationCallAnalyticsInput,
) -> Result<GenerationCallAnalytics, AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await?;
    state
        .admin_read_application()
        .generation_call_analytics(caller_user_id, input)
        .await
}
