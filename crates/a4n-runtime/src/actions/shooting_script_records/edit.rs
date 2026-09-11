use super::contracts::{EditChapterShootingScriptInput, EditedChapterShootingScriptView};
use crate::AppState;

pub async fn edit_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: EditChapterShootingScriptInput,
) -> Result<EditedChapterShootingScriptView, String> {
    state
        .shooting_script_application()
        .edit(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
