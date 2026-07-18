use crate::AppState;
use crate::actions::access as project_access;
use crate::actions::outline_records::preview::{outline_preview, single_request_preview};
use crate::actions::outline_records::types::{
    GeneratePartsInput, RegenerateLastPartInput, RegeneratePartsInput,
};
use crate::actions::request_preview::ModelRequestPreview;
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;
use a4n_core::error::AppError;

pub async fn preview_generate_parts_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePartsInput,
) -> Result<ModelRequestPreview, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let steps = state
        .orchestrator
        .preview_generate_parts_requests(
            input.project_id,
            input.part_count,
            input.chapters_per_part,
            false,
            state,
            &variables,
        )
        .await?;
    outline_preview(
        state,
        "part_outline_generation",
        "分部大纲生成请求预览",
        steps,
    )
    .await
}

pub async fn preview_regenerate_parts_request(
    state: &AppState,
    user_id: i64,
    input: RegeneratePartsInput,
) -> Result<ModelRequestPreview, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let steps = state
        .orchestrator
        .preview_generate_parts_requests(
            input.project_id,
            input.part_count,
            input.chapters_per_part,
            true,
            state,
            &variables,
        )
        .await?;
    outline_preview(
        state,
        "part_outline_regeneration",
        "分部大纲重生成请求预览",
        steps,
    )
    .await
}

pub async fn preview_regenerate_last_part_request(
    state: &AppState,
    user_id: i64,
    input: RegenerateLastPartInput,
) -> Result<ModelRequestPreview, AppError> {
    project_access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let request = state
        .orchestrator
        .preview_regenerate_last_part_request(input.project_id, state, &variables)
        .await?;
    single_request_preview(
        state,
        "part_outline_regenerate_last",
        "最后一部分部大纲重生成请求预览",
        "regenerate_last_part",
        "重新生成最后一部",
        "part_outline_single",
        request,
    )
    .await
}
