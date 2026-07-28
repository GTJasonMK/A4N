use crate::AppState;
use crate::actions::access as project_access;
use crate::actions::outline_records::types::{
    GenerationProgressSummary, PartOutlineSummary, part_outline_summary,
};
use a4n_core::error::AppError;

pub async fn list_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    project_access::verify_project_access(state, user_id, project_id).await?;
    let parts = state
        .project_outline_records()
        .part_outlines_for_project(project_id)
        .await?;
    Ok(parts.iter().map(part_outline_summary).collect())
}

pub async fn delete_latest_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<(), AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    state
        .outline()
        .delete_latest_part_outline_records_validated(project_id, count)
        .await
}

pub async fn get_generation_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<GenerationProgressSummary, AppError> {
    state
        .outline()
        .generation_progress_for_user_validated(user_id, project_id)
        .await
}
