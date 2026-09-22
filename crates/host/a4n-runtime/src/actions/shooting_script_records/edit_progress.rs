use crate::AppState;
use a4n_api_contract::shooting_script::{
    EditChapterShootingScriptProgressInput, EditedChapterShootingScriptProgressView,
};

pub async fn edit_chapter_shooting_script_progress(
    state: &AppState,
    user_id: i64,
    input: EditChapterShootingScriptProgressInput,
) -> Result<EditedChapterShootingScriptProgressView, String> {
    state
        .shooting_script_record_mutation_application()
        .edit_progress(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
