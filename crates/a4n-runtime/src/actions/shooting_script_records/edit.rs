use super::contracts::{
    EditChapterShootingScriptInput, EditedChapterShootingScriptView, contract_view,
};
use crate::AppState;

pub async fn edit_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: EditChapterShootingScriptInput,
) -> Result<EditedChapterShootingScriptView, String> {
    let edited = state
        .orchestrator()
        .edit_current_chapter_shooting_script_for_user_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(EditedChapterShootingScriptView {
        shooting_script: contract_view(
            edited.shooting_script.record,
            edited.shooting_script.script,
        ),
        invalidated_project_ids: edited.invalidated_project_ids,
    })
}
