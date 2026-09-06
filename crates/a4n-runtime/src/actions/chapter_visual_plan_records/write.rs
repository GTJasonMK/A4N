use crate::AppState;

use super::{
    ChapterVisualPlanRecordInput, ChapterVisualPlanView, ConfirmedChapterVisualPlanView,
    SaveChapterVisualPlanDraftInput, contracts::plan_view,
};

pub async fn save_chapter_visual_plan_draft(
    state: &AppState,
    user_id: i64,
    input: SaveChapterVisualPlanDraftInput,
) -> Result<ChapterVisualPlanView, String> {
    state
        .orchestrator()
        .save_chapter_visual_plan_draft_for_user_validated(user_id, input)
        .await
        .map(plan_view)
        .map_err(|error| error.to_string())
}

pub async fn confirm_chapter_visual_plan(
    state: &AppState,
    user_id: i64,
    input: ChapterVisualPlanRecordInput,
) -> Result<ConfirmedChapterVisualPlanView, String> {
    let outcome = state
        .orchestrator()
        .confirm_chapter_visual_plan_for_user_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ConfirmedChapterVisualPlanView {
        visual_plan: plan_view(outcome.visual_plan),
        invalidated_project_ids: outcome.invalidated_project_ids,
    })
}

pub async fn discard_chapter_visual_plan_draft(
    state: &AppState,
    user_id: i64,
    input: ChapterVisualPlanRecordInput,
) -> Result<(), String> {
    state
        .orchestrator()
        .discard_chapter_visual_plan_draft_for_user_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
