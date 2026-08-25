use crate::AppState;

use super::{ChapterVisualPlanSetView, ChapterVisualPlanSourceInput, contracts::plan_set_view};

pub async fn get_chapter_visual_plan(
    state: &AppState,
    user_id: i64,
    input: ChapterVisualPlanSourceInput,
) -> Result<ChapterVisualPlanSetView, String> {
    state
        .orchestrator()
        .current_chapter_visual_plan_for_user_validated(user_id, input)
        .await
        .map(plan_set_view)
        .map_err(|error| error.to_string())
}
