use super::super::{GeneratePartChapterOutlinesInput, ensure_part_access, outline_preview};
use crate::{AppState, actions::request_preview::ModelRequestPreview};
use a4n_core::error::AppError;

pub async fn preview_generate_part_chapter_outlines_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePartChapterOutlinesInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_part_access(
        state.orchestrator(),
        state.orchestrator(),
        user_id,
        input.project_id,
        input.part_id,
    )
    .await?;
    let steps = state
        .orchestrator()
        .preview_generate_part_chapter_outlines_validated(input)
        .await?;
    outline_preview(
        state,
        "part_chapter_outline_generation",
        "本部章节大纲生成请求预览",
        steps,
    )
    .await
}
