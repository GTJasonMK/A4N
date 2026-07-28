use crate::{
    AppState,
    actions::admin_records::{AdminPermission, require_admin_permission},
};
use a4n_core::error::AppError;
pub use a4n_orchestrator::admin::{GenerationCallAnalytics, GenerationCallAnalyticsInput};

pub async fn get_generation_call_analytics_for_admin(
    state: &AppState,
    caller_user_id: i64,
    input: GenerationCallAnalyticsInput,
) -> Result<GenerationCallAnalytics, AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await?;
    state.admin().generation_call_analytics(input).await
}
