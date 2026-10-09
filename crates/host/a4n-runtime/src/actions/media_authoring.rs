use crate::AppState;
use a4n_api_contract::media_authoring::*;
use a4n_error::AppError;

pub async fn get_media_authoring_workspace(
    state: &AppState,
    user_id: i64,
    input: MediaAuthoringInput,
) -> Result<MediaAuthoringWorkspace, AppError> {
    state
        .media_authoring_application()
        .workspace(user_id, input)
        .await
}
pub async fn save_media_authoring_document(
    state: &AppState,
    user_id: i64,
    input: SaveAuthoringDocumentInput,
) -> Result<AuthoringDocumentRecord, AppError> {
    state
        .media_authoring_application()
        .save(user_id, input)
        .await
}
pub async fn adopt_media_authoring_document(
    state: &AppState,
    user_id: i64,
    input: AdoptAuthoringDocumentInput,
) -> Result<AuthoringDocumentRecord, AppError> {
    state
        .media_authoring_application()
        .adopt(user_id, input)
        .await
}
pub async fn delete_media_authoring_document(
    state: &AppState,
    user_id: i64,
    input: DeleteAuthoringDocumentInput,
) -> Result<(), AppError> {
    state
        .media_authoring_application()
        .delete(user_id, input)
        .await
}
pub async fn list_video_adaptation_references(
    state: &AppState,
    user_id: i64,
    input: MediaAuthoringInput,
) -> Result<Vec<VideoScriptReferenceCandidate>, AppError> {
    state
        .media_authoring_application()
        .video_references(user_id, input)
        .await
}
pub async fn generate_media_adaptation_plan(
    state: &AppState,
    user_id: i64,
    input: GenerateAdaptationPlanInput,
    stream_reporter: Option<&MediaAuthoringStreamReporter<'_>>,
) -> Result<AuthoringDocumentRecord, AppError> {
    state
        .media_authoring_application()
        .generate(user_id, input, state, stream_reporter)
        .await
}
pub async fn preview_media_adaptation_plan_request(
    state: &AppState,
    user_id: i64,
    input: GenerateAdaptationPlanInput,
) -> Result<a4n_api_contract::request_preview::ModelRequestPreview, AppError> {
    use crate::actions::request_preview::{ChatStatusPreviewStep, chat_status_preview_for_state};
    let request = state
        .media_authoring_application()
        .preview(user_id, input, state)
        .await?;
    chat_status_preview_for_state(
        state,
        "media_adaptation_plan",
        "改编计划请求预览",
        vec![ChatStatusPreviewStep::llm(
            "adaptation_plan",
            "改编计划",
            Some("chapter_media_adaptation_plan".into()),
            "ready",
            None,
            Some(serde_json::from_value(request)?),
        )],
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
