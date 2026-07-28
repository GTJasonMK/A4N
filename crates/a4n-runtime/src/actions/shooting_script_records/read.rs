use a4n_orchestrator::shooting_script::ChapterShootingScriptSourceInput;

use super::contracts::{ChapterShootingScriptView, GetChapterShootingScriptInput, contract_view};
use crate::AppState;

pub async fn get_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: GetChapterShootingScriptInput,
) -> Result<Option<ChapterShootingScriptView>, String> {
    let resolved = state
        .shooting_script()
        .current_chapter_shooting_script_for_user_validated(
            user_id,
            ChapterShootingScriptSourceInput {
                project_id: input.project_id,
                source_chapter_id: input.source_chapter_id,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    resolved
        .map(|resolved| {
            contract_view(
                resolved.shooting_script.record,
                resolved.shooting_script.script,
            )
        })
        .transpose()
}
