use crate::{
    AppState,
    actions::{
        outline_records::{
            access::ensure_part_access, preview::single_request_preview,
            types::RegeneratePartOutlineInput,
        },
        request_preview::ModelRequestPreview,
    },
};
use a4n_error::AppError;

pub async fn preview_regenerate_part_outline_request(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartOutlineInput,
) -> Result<ModelRequestPreview, AppError> {
    ensure_part_access(
        state.outline_read_application(),
        user_id,
        input.project_id,
        input.part_id,
    )
    .await?;
    let part_id = input.part_id;
    let request = state
        .outline_part_mutation_preview_application()
        .preview_regenerate_part_outline(input)
        .await?;
    single_request_preview(
        state,
        "part_outline_regenerate_one",
        "指定分部大纲重生成请求预览",
        "regenerate_part_outline",
        format!("重新生成分部 #{part_id}"),
        "part_outline_single",
        request,
    )
    .await
}
