use crate::AppState;
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanReadResult, ChapterVisualPlanValidationResult, ReadChapterVisualPlanInput,
    ValidateChapterVisualPlanInput,
};
use a4n_error::AppError;

use super::{ChapterVisualPlanSetView, ChapterVisualPlanSourceInput};

pub async fn get_chapter_visual_plan(
    state: &AppState,
    user_id: i64,
    input: ChapterVisualPlanSourceInput,
) -> Result<ChapterVisualPlanSetView, String> {
    state
        .chapter_visual_plan_record_read_application()
        .get(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn read_chapter_visual_plan(
    state: &AppState,
    user_id: i64,
    input: ReadChapterVisualPlanInput,
) -> Result<ChapterVisualPlanReadResult, AppError> {
    state
        .chapter_visual_plan_record_read_application()
        .read(user_id, input)
        .await
}

pub async fn validate_chapter_visual_plan(
    state: &AppState,
    user_id: i64,
    input: ValidateChapterVisualPlanInput,
) -> Result<ChapterVisualPlanValidationResult, AppError> {
    state
        .chapter_visual_plan_record_read_application()
        .validate(user_id, input)
        .await
}
