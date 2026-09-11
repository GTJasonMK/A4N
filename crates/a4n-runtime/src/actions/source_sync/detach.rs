use crate::AppState;
use a4n_error::AppError;

pub async fn detach_derived_project(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<(), AppError> {
    state
        .source_sync_application()
        .detach(
            user_id,
            a4n_api_contract::source_sync::SourceSyncProjectInput { derived_project_id },
        )
        .await
}
