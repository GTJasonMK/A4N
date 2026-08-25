use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use a4n_core::error::AppError;
use a4n_orchestrator::manga::MangaPipelinePreviewApplicationInput;

pub async fn preview_manga_pipeline_requests(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    resume_from_checkpoint: bool,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_or_derived_family_editor(state, user_id, project_id).await?;
    let prompt_renderer = state;
    let steps = state
        .orchestrator()
        .prepare_manga_pipeline_preview_validated(MangaPipelinePreviewApplicationInput {
            project_id,
            chapter_number,
            prompt_renderer,
            resume_from_checkpoint,
        })
        .await?;
    chat_status_preview_for_state(
        state,
        "manga_pipeline",
        format!("第 {chapter_number} 章漫画流水线请求预览"),
        steps.into_iter().map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        }),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
