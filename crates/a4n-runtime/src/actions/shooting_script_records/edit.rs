use a4n_orchestrator::shooting_script::{
    ChapterShootingScriptSourceInput, EditChapterShootingScriptInput,
};

use super::contracts::{
    EditChapterShootingScriptRecordInput, EditedChapterShootingScriptView, contract_view,
    domain_script,
};
use crate::AppState;

pub async fn edit_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: EditChapterShootingScriptRecordInput,
) -> Result<EditedChapterShootingScriptView, String> {
    let script = domain_script(input.script)?;
    let edited = state
        .shooting_script()
        .edit_current_chapter_shooting_script_for_user_validated(
            user_id,
            ChapterShootingScriptSourceInput {
                project_id: input.project_id,
                source_chapter_id: input.source_chapter_id,
            },
            EditChapterShootingScriptInput {
                record_id: input.record_id,
                expected_revision: input.expected_revision,
                script,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(EditedChapterShootingScriptView {
        shooting_script: contract_view(
            edited.shooting_script.record,
            edited.shooting_script.script,
        )?,
        invalidated_project_ids: edited.invalidated_project_ids,
    })
}
