use crate::AppState;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use a4n_error::AppError;

pub async fn preview_manga_pipeline_requests(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    page_number: Option<u32>,
    resume_from_checkpoint: bool,
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
) -> Result<ModelRequestPreview, AppError> {
    let prompt_renderer = state;
    let steps = state
        .manga_pipeline_preview_application()
        .preview(
            user_id,
            project_id,
            chapter_number,
            page_number,
            resume_from_checkpoint,
            stop_after,
            prompt_renderer,
        )
        .await?;
    chat_status_preview_for_state(
        state,
        "manga_pipeline",
        format!("第 {chapter_number} 章漫画流水线请求预览"),
        steps
            .into_iter()
            .map(|step| {
                let request = step
                    .request
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(|error| format!("漫画预览请求格式无效: {error}"))?;
                Ok(ChatStatusPreviewStep::llm(
                    step.key,
                    step.title,
                    step.prompt_name,
                    step.state,
                    step.note,
                    request,
                ))
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(AppError::InvalidArgument)?,
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
