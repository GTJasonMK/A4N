use super::super::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ContinuePartOutlinesInput,
    GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
    OutlineTaskScope, RegenerateChapterOutlineInput, RegenerateLastPartInput,
    RegeneratePartOutlineInput, RegeneratePartsInput,
};
use super::{
    batch_parts_progress_total, collect_unique_parts, ensure_chapter_outline_access,
    ensure_part_access, outline_project_task_scope, part_chapter_count, progress_total,
};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn generate_parts_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GeneratePartsInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(state, user_id, input.project_id, 3).await
}

pub async fn regenerate_parts_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegeneratePartsInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(state, user_id, input.project_id, 3).await
}

pub async fn regenerate_last_part_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegenerateLastPartInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(state, user_id, input.project_id, 3).await
}

pub async fn continue_part_outlines_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ContinuePartOutlinesInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(state, user_id, input.project_id, 3).await
}

pub async fn regenerate_part_outline_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegeneratePartOutlineInput,
) -> Result<OutlineTaskScope, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total: 3,
    })
}

pub async fn generate_part_chapter_outlines_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GeneratePartChapterOutlinesInput,
) -> Result<OutlineTaskScope, AppError> {
    let part = ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total: progress_total(part_chapter_count(part.start_chapter, part.end_chapter)?)?,
    })
}

pub async fn batch_generate_part_outlines_task_scope(
    state: &AppState,
    user_id: i64,
    input: &BatchGeneratePartOutlinesInput,
) -> Result<OutlineTaskScope, AppError> {
    let parts = collect_unique_parts(state, user_id, input.project_id, &input.part_ids).await?;
    let total = batch_parts_progress_total(&parts)?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total,
    })
}

pub async fn apply_beat_mapping_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ApplyBeatMappingInput,
) -> Result<OutlineTaskScope, AppError> {
    ensure_part_access(state, user_id, input.project_id, input.part_id).await?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total: 3,
    })
}

pub async fn generate_chapter_outlines_by_count_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterOutlinesByCountInput,
) -> Result<OutlineTaskScope, AppError> {
    outline_project_task_scope(
        state,
        user_id,
        input.project_id,
        progress_total(input.count)?,
    )
    .await
}

pub async fn regenerate_chapter_outline_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegenerateChapterOutlineInput,
) -> Result<OutlineTaskScope, AppError> {
    ensure_chapter_outline_access(state, user_id, input.project_id, input.outline_id).await?;
    Ok(OutlineTaskScope {
        project_id: input.project_id,
        total: 3,
    })
}
