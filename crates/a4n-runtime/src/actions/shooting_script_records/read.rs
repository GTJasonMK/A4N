use super::contracts::{
    ChapterShootingScriptSourceInput, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptView,
};
use crate::AppState;

pub async fn get_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: ChapterShootingScriptSourceInput,
) -> Result<Option<ChapterShootingScriptView>, String> {
    state
        .shooting_script_current_application()
        .current(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn get_chapter_shooting_script_unit_progress(
    state: &AppState,
    user_id: i64,
    input: ChapterShootingScriptSourceInput,
) -> Result<Option<ChapterShootingScriptUnitProgressView>, String> {
    state
        .shooting_script_unit_progress_application()
        .unit_progress(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
