use super::contracts::{
    ChapterShootingScriptSourceInput, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptView, contract_view, progress_contract_view,
};
use crate::AppState;

pub async fn get_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: ChapterShootingScriptSourceInput,
) -> Result<Option<ChapterShootingScriptView>, String> {
    let resolved = state
        .orchestrator()
        .current_chapter_shooting_script_for_user_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(resolved.map(|resolved| {
        contract_view(
            resolved.shooting_script.record,
            resolved.shooting_script.script,
        )
    }))
}

pub async fn get_chapter_shooting_script_unit_progress(
    state: &AppState,
    user_id: i64,
    input: ChapterShootingScriptSourceInput,
) -> Result<Option<ChapterShootingScriptUnitProgressView>, String> {
    Ok(state
        .orchestrator()
        .current_chapter_shooting_script_unit_progress_for_user_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())?
        .map(progress_contract_view))
}
