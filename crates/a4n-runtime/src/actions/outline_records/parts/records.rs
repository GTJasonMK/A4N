use crate::AppState;
use crate::actions::outline_records::outline_material_sync_warnings;
use crate::actions::outline_records::types::{
    DeleteLatestOutlineRecordsInput, OutlineGenerationProgressSummary, OutlineMutationResult,
    PartOutlineSummary, SavePartOutlineInput, SavePartOutlineResult,
};
use a4n_error::AppError;

pub async fn list_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PartOutlineSummary>, AppError> {
    state
        .outline_read_application()
        .list_parts(user_id, project_id)
        .await
}

pub async fn delete_latest_parts(
    state: &AppState,
    user_id: i64,
    input: DeleteLatestOutlineRecordsInput,
) -> Result<OutlineMutationResult, AppError> {
    let project_id = input.project_id;
    let mut result = state
        .outline_mutation_application()
        .delete_latest_parts(user_id, input)
        .await?;
    result.warnings = outline_material_sync_warnings(state, project_id).await;
    Ok(result)
}

pub async fn save_part_outline(
    state: &AppState,
    user_id: i64,
    input: SavePartOutlineInput,
) -> Result<SavePartOutlineResult, AppError> {
    let project_id = input.project_id;
    let mut result = state
        .outline_mutation_application()
        .save_part(user_id, input)
        .await?;
    result.warnings = outline_material_sync_warnings(state, project_id).await;
    Ok(result)
}

pub async fn get_generation_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<OutlineGenerationProgressSummary, AppError> {
    state
        .outline_read_application()
        .generation_progress(user_id, project_id)
        .await
}
