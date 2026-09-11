use super::super::{
    ChapterOutlineSummary, DeleteLatestOutlineRecordsInput, OutlineMutationResult,
    SaveChapterOutlineInput, SaveChapterOutlineResult, UpdateChapterOutlineInput,
    UpdateChapterOutlineResult, outline_material_sync_warnings,
};
use crate::AppState;
use a4n_error::AppError;

pub async fn list_chapter_outlines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    state
        .outline_read_application()
        .list_chapters(user_id, project_id)
        .await
}

pub async fn delete_latest_chapter_outlines(
    state: &AppState,
    user_id: i64,
    input: DeleteLatestOutlineRecordsInput,
) -> Result<OutlineMutationResult, AppError> {
    let project_id = input.project_id;
    let mut result = state
        .outline_mutation_application()
        .delete_latest_chapters(user_id, input)
        .await?;
    result.warnings = outline_material_sync_warnings(state, project_id).await;
    Ok(result)
}

pub async fn save_chapter_outline(
    state: &AppState,
    user_id: i64,
    input: SaveChapterOutlineInput,
) -> Result<SaveChapterOutlineResult, AppError> {
    let project_id = input.project_id;
    let mut result = state
        .outline_mutation_application()
        .save_chapter(user_id, input)
        .await?;
    result.warnings = outline_material_sync_warnings(state, project_id).await;
    Ok(result)
}

pub async fn update_chapter_outline(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterOutlineInput,
) -> Result<UpdateChapterOutlineResult, AppError> {
    let project_id = input.project_id;
    let mut result = state
        .outline_mutation_application()
        .update_chapter(user_id, input)
        .await?;
    result.warnings = outline_material_sync_warnings(state, project_id).await;
    Ok(result)
}
