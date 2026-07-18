use super::access;
use crate::AppState;
use a4n_core::{error::AppError, models::project::novel_project};

pub async fn detach_derived_project(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<novel_project::Model, AppError> {
    let family =
        access::verify_derived_project_family_editor_context(state, user_id, derived_project_id)
            .await?;
    state
        .orchestrator
        .detach_derived_project_record(family.project)
        .await
}
