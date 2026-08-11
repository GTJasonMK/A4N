use super::super::{
    ChapterOutlineSummary, DeleteLatestOutlineRecordsInput, OutlineMutationResult,
    SaveChapterOutlineInput, SaveChapterOutlineResult, UpdateChapterOutlineInput,
    UpdateChapterOutlineResult, chapter_outline_summary, outline_material_sync_warnings,
    project_outline_delete_input,
};
use crate::AppState;
use crate::actions::access as project_access;
use a4n_core::error::AppError;
use a4n_orchestrator::project::{ManualChapterOutlineSaveInput, ProjectChapterOutlineUpdateInput};

pub async fn list_chapter_outlines(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterOutlineSummary>, AppError> {
    project_access::verify_project_access(state, user_id, project_id).await?;
    let outlines = state
        .orchestrator()
        .chapter_outlines_for_project(project_id)
        .await?;
    let outline_ids = outlines
        .iter()
        .map(|outline| outline.id)
        .collect::<Vec<_>>();
    let source_kinds = state
        .orchestrator()
        .chapter_outline_source_kinds_for_ids(&outline_ids)
        .await?;
    Ok(outlines
        .iter()
        .map(|outline| {
            chapter_outline_summary(outline, source_kinds.get(&outline.id).map(String::as_str))
        })
        .collect())
}

pub async fn delete_latest_chapter_outlines(
    state: &AppState,
    user_id: i64,
    input: DeleteLatestOutlineRecordsInput,
) -> Result<OutlineMutationResult, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let project_id = input.project_id;
    state
        .orchestrator()
        .delete_latest_chapter_outline_records_validated(project_outline_delete_input(input))
        .await?;
    Ok(OutlineMutationResult {
        warnings: outline_material_sync_warnings(state, project_id).await,
    })
}

pub async fn save_chapter_outline(
    state: &AppState,
    user_id: i64,
    input: SaveChapterOutlineInput,
) -> Result<SaveChapterOutlineResult, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let project_id = input.project_id;
    let saved = state
        .orchestrator()
        .save_manual_chapter_outline_validated(ManualChapterOutlineSaveInput {
            project_id,
            outline_id: input.outline_id,
            expected_updated_at: input.expected_updated_at,
            chapter_number: input.chapter_number,
            title: input.title,
            summary: input.summary,
        })
        .await?;
    Ok(SaveChapterOutlineResult {
        outline: chapter_outline_summary(&saved, None),
        warnings: outline_material_sync_warnings(state, project_id).await,
    })
}

pub async fn update_chapter_outline(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterOutlineInput,
) -> Result<UpdateChapterOutlineResult, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let updated = state
        .orchestrator()
        .update_chapter_outline_validated(ProjectChapterOutlineUpdateInput {
            project_id: input.project_id,
            outline_id: input.outline_id,
            expected_updated_at: input.expected_updated_at,
            title: input.title,
            summary: input.summary,
            beats: input.beats,
            key_scenes: input.key_scenes,
        })
        .await?;
    Ok(UpdateChapterOutlineResult {
        outline: chapter_outline_summary(&updated, None),
        warnings: outline_material_sync_warnings(state, input.project_id).await,
    })
}
