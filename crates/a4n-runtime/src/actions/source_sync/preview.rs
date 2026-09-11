use super::SourceSyncPlanSummary;
use crate::AppState;
use a4n_error::AppError;

pub async fn preview_source_sync(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<SourceSyncPlanSummary, AppError> {
    state
        .source_sync_application()
        .preview(
            user_id,
            a4n_api_contract::source_sync::SourceSyncProjectInput { derived_project_id },
        )
        .await
}
