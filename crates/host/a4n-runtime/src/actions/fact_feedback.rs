use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::{AppState, workflow::RuntimeChapterAnalysisHostPort};
use a4n_api_contract::fact_feedback::{ProjectFactFeedbackSummary, RunProjectFactFeedbackInput};
pub use a4n_chapter_analysis_host_contract::ChapterAnalysisApplicationProgress;
use a4n_error::AppError;

pub async fn get_project_fact_feedback(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectFactFeedbackSummary, AppError> {
    state
        .project_fact_feedback_application()
        .feedback_summary(user_id, project_id)
        .await
}

pub async fn preflight_project_fact_feedback(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    crate::actions::access::verify_project_editor(state, user_id, project_id).await
}

pub async fn run_project_fact_feedback(
    state: &AppState,
    user_id: i64,
    input: RunProjectFactFeedbackInput,
    progress: ChapterAnalysisApplicationProgress<'_>,
) -> Result<ProjectFactFeedbackSummary, AppError> {
    let host = RuntimeChapterAnalysisHostPort::new(state);
    let trace = crate::actions::character_ai::RuntimeCharacterProfileSyncStageTracePort::new(
        state.log_dir(),
    );
    state
        .project_fact_feedback_application()
        .run_feedback(user_id, input, state, &host, progress, Some(&trace))
        .await
}

pub async fn preview_project_fact_feedback_request(
    state: &AppState,
    user_id: i64,
    input: RunProjectFactFeedbackInput,
) -> Result<ModelRequestPreview, AppError> {
    let preview = state
        .project_fact_feedback_application()
        .preview_feedback(user_id, input, state)
        .await?;
    chat_status_preview_for_state(
        state,
        "project_fact_feedback",
        preview.title,
        preview.steps.into_iter().map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                Some(step.prompt_name),
                "ready",
                None,
                Some(step.request),
            )
        }),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
