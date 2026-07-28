use super::super::{BatchGeneratePartOutlinesInput, outline_preview};
use crate::{AppState, actions::request_preview::ModelRequestPreview};
use a4n_core::error::AppError;

pub async fn preview_batch_generate_part_outlines_request(
    state: &AppState,
    user_id: i64,
    input: BatchGeneratePartOutlinesInput,
) -> Result<ModelRequestPreview, AppError> {
    state
        .collaboration()
        .verify_project_permission(
            input.project_id,
            user_id,
            a4n_orchestrator::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
    let all_steps = state
        .outline()
        .preview_batch_generate_part_outlines_validated(input)
        .await?;
    outline_preview(
        state,
        "batch_part_chapter_outline_generation",
        "批量章节大纲生成请求预览",
        all_steps,
    )
    .await
}
