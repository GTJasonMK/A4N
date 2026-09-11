use crate::AppState;
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanEditResult, EditChapterVisualPlanDraftInput,
};
use a4n_error::AppError;

use super::{
    ChapterVisualPlanRecordInput, ChapterVisualPlanView, ConfirmedChapterVisualPlanView,
    SaveChapterVisualPlanDraftInput,
};

pub async fn save_chapter_visual_plan_draft(
    state: &AppState,
    user_id: i64,
    input: SaveChapterVisualPlanDraftInput,
) -> Result<ChapterVisualPlanView, String> {
    state
        .chapter_visual_plan_mutation_application()
        .save_draft(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn edit_chapter_visual_plan_draft(
    state: &AppState,
    user_id: i64,
    input: EditChapterVisualPlanDraftInput,
) -> Result<ChapterVisualPlanEditResult, AppError> {
    state
        .chapter_visual_plan_mutation_application()
        .edit_draft(user_id, input)
        .await
}

pub async fn confirm_chapter_visual_plan(
    state: &AppState,
    user_id: i64,
    input: ChapterVisualPlanRecordInput,
) -> Result<ConfirmedChapterVisualPlanView, String> {
    state
        .chapter_visual_plan_mutation_application()
        .confirm(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn discard_chapter_visual_plan_draft(
    state: &AppState,
    user_id: i64,
    input: ChapterVisualPlanRecordInput,
) -> Result<(), String> {
    state
        .chapter_visual_plan_mutation_application()
        .discard(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
