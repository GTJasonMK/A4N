use crate::AppState;
use crate::actions::access as project_access;
use crate::actions::outline_records::outline_material_sync_warnings;
use crate::actions::outline_records::types::{
    DeleteLatestOutlineRecordsInput, OutlineGenerationProgressSummary, OutlineMutationResult,
    PartOutlineSummary, SavePartOutlineInput, SavePartOutlineResult, part_outline_summary,
};
use a4n_core::error::AppError;

pub async fn list_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    project_access::verify_project_access(state, user_id, project_id).await?;
    let parts = state
        .orchestrator()
        .part_outlines_for_project(project_id)
        .await?;
    Ok(parts.iter().map(part_outline_summary).collect())
}

pub async fn delete_latest_parts(
    state: &AppState,
    user_id: i64,
    input: DeleteLatestOutlineRecordsInput,
) -> Result<OutlineMutationResult, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let project_id = input.project_id;
    state
        .orchestrator()
        .delete_latest_part_outline_records_validated(input)
        .await?;
    Ok(OutlineMutationResult {
        warnings: outline_material_sync_warnings(state, project_id).await,
    })
}

pub async fn save_part_outline(
    state: &AppState,
    user_id: i64,
    input: SavePartOutlineInput,
) -> Result<SavePartOutlineResult, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let project_id = input.project_id;
    let saved = state
        .orchestrator()
        .save_manual_part_outline_validated(input)
        .await?;
    Ok(SavePartOutlineResult {
        part: part_outline_summary(&saved),
        warnings: outline_material_sync_warnings(state, project_id).await,
    })
}

pub async fn get_generation_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<OutlineGenerationProgressSummary, AppError> {
    state
        .orchestrator()
        .outline_generation_progress_for_user_validated(user_id, project_id)
        .await
}
