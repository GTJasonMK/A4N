use super::super::{
    RegenerateChapterOutlineInput, ensure_chapter_outline_access, single_request_preview,
};
use crate::{AppState, actions::request_preview::ModelRequestPreview};
use a4n_error::AppError;

pub async fn preview_regenerate_chapter_outline_request(
    state: &AppState,
    user_id: i64,
    input: RegenerateChapterOutlineInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_chapter_outline_access(
        state.outline_read_application(),
        user_id,
        input.project_id,
        input.outline_id,
    )
    .await?;
    let outline_id = input.outline_id;
    let request = state
        .outline_chapter_regeneration_preview_application()
        .preview_regenerate_chapter_outline(input)
        .await?;
    single_request_preview(
        state,
        "chapter_outline_regenerate_one",
        "章节大纲重生成请求预览",
        "regenerate_chapter_outline",
        format!("重新生成章节大纲 #{outline_id}"),
        "outline",
        request,
    )
    .await
}
