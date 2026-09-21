use super::super::{GenerateChapterOutlinesByCountInput, outline_preview};
use crate::{
    AppState,
    actions::{access as project_access, request_preview::ModelRequestPreview},
};
use a4n_error::AppError;

pub async fn preview_generate_chapter_outlines_by_count_request(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterOutlinesByCountInput,
) -> Result<ModelRequestPreview, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let steps = state
        .outline_chapter_count_generation_preview_application()
        .preview_generate_chapter_outlines_by_count(input)
        .await?;
    outline_preview(
        state,
        "chapter_outline_generation_by_count",
        "章节大纲追加生成请求预览",
        steps,
    )
    .await
}
