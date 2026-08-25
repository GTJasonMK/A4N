use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
};

use super::{
    ChapterVisualPlanProposalView, ChapterVisualPlanView, GenerateChapterVisualPlanDraftInput,
    ProposeChapterVisualPlanRevisionInput, contracts::plan_view,
};

type ProgressReporter =
    dyn Fn(u32, u32, &str) -> Result<(), a4n_core::error::AppError> + Send + Sync;

pub async fn preview_chapter_visual_plan_request(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterVisualPlanDraftInput,
) -> Result<ModelRequestPreview, String> {
    let prepared = state
        .orchestrator()
        .prepare_chapter_visual_plan_request_for_user_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    let step = chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: "chapter_visual_plan_outline".into(),
            title: prepared.title.clone(),
            prompt_name: Some("chapter_visual_plan_outline".into()),
            state: "ready".into(),
            note: None,
            request: prepared.request,
        },
    )
    .await?;
    Ok(model_request_preview(
        "chapter_visual_plan",
        format!("{}请求预览", prepared.title),
        vec![step],
    ))
}

pub async fn generate_chapter_visual_plan_draft(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterVisualPlanDraftInput,
    progress: &ProgressReporter,
    stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
    cancel_token: Option<&tokio_util::sync::CancellationToken>,
) -> Result<ChapterVisualPlanView, String> {
    state
        .orchestrator()
        .generate_chapter_visual_plan_draft_for_user_validated(
            user_id,
            input,
            progress,
            stream_reporter,
            cancel_token,
        )
        .await
        .map(plan_view)
        .map_err(|error| error.to_string())
}

pub async fn propose_chapter_visual_plan_revision(
    state: &AppState,
    user_id: i64,
    input: ProposeChapterVisualPlanRevisionInput,
    progress: &ProgressReporter,
    stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
    cancel_token: Option<&tokio_util::sync::CancellationToken>,
) -> Result<ChapterVisualPlanProposalView, String> {
    let proposal = state
        .orchestrator()
        .propose_chapter_visual_plan_revision_for_user_validated(
            user_id,
            input,
            progress,
            stream_reporter,
            cancel_token,
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(ChapterVisualPlanProposalView {
        record_id: proposal.record_id,
        base_revision: proposal.base_revision,
        proposal: proposal.proposal,
    })
}
