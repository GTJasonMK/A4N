use a4n_orchestrator::shooting_script::ChapterShootingScriptSourceInput;

use super::contracts::{ChapterShootingScriptView, GetChapterShootingScriptInput};
use crate::{AppState, actions::access};

pub async fn get_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: GetChapterShootingScriptInput,
) -> Result<Option<ChapterShootingScriptView>, String> {
    access::verify_project_or_derived_family_access(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let resolved = state
        .orchestrator
        .current_chapter_shooting_script(ChapterShootingScriptSourceInput {
            project_id: input.project_id,
            source_chapter_id: input.source_chapter_id,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(resolved.map(|resolved| ChapterShootingScriptView {
        record: resolved.shooting_script.record,
        script: resolved.shooting_script.script,
    }))
}
