use crate::AppState;
use a4n_api_contract::shooting_script::DeleteChapterShootingScriptInput;

pub async fn delete_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: DeleteChapterShootingScriptInput,
) -> Result<(), String> {
    state
        .shooting_script_record_mutation_application()
        .delete(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
