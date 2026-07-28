use crate::AppState;
use a4n_core::{error::AppError, models::project::novel_project};

pub async fn detach_derived_project(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<novel_project::Model, AppError> {
    state
        .source_sync()
        .detach_derived_project_for_user_validated(user_id, derived_project_id)
        .await
}
