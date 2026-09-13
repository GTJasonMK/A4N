use crate::{
    AppState,
    actions::{
        access as project_access,
        outline_records::{preview::single_request_preview, types::ContinuePartOutlinesInput},
        request_preview::ModelRequestPreview,
    },
};
use a4n_error::AppError;

pub async fn preview_continue_part_outlines_request(
    state: &AppState,
    user_id: i64,
    input: ContinuePartOutlinesInput,
) -> Result<ModelRequestPreview, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let part_number = input.part_number;
    let request = state
        .outline_part_mutation_preview_application()
        .preview_continue_part_outlines(input)
        .await?;
    single_request_preview(
        state,
        "part_outline_continue",
        format!("第 {part_number} 部续作请求预览"),
        "continue_part_outline",
        format!("续作第 {part_number} 部"),
        "part_outline_single",
        request,
    )
    .await
}
