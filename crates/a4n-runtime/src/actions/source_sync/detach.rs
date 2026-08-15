use crate::AppState;
use a4n_core::error::AppError;

pub async fn detach_derived_project(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<(), AppError> {
    state
        .orchestrator()
        .detach_derived_project_for_user_validated(user_id, derived_project_id)
        .await
}
