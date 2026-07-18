use a4n_orchestrator::shooting_script::{
    ChapterShootingScriptSourceInput, EditChapterShootingScriptInput,
};

use super::contracts::{
    ChapterShootingScriptView, EditChapterShootingScriptRecordInput,
    EditedChapterShootingScriptView,
};
use crate::{AppState, actions::access};

pub async fn edit_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: EditChapterShootingScriptRecordInput,
) -> Result<EditedChapterShootingScriptView, String> {
    access::verify_project_or_derived_family_editor(state, user_id, input.project_id)
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
    let existing = resolved
        .map(|resolved| resolved.shooting_script)
        .ok_or_else(|| "当前章节尚无可编辑的共享拍摄剧本".to_string())?;
    if existing.record.id != input.record_id {
        return Err(format!(
            "拍摄剧本与当前章节来源不匹配: expected_record_id={}, actual_record_id={}",
            existing.record.id, input.record_id
        ));
    }
    let edited = state
        .orchestrator
        .edit_chapter_shooting_script(EditChapterShootingScriptInput {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
            script: input.script,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(EditedChapterShootingScriptView {
        shooting_script: ChapterShootingScriptView {
            record: edited.shooting_script.record,
            script: edited.shooting_script.script,
        },
        invalidated_project_ids: edited.invalidated_project_ids,
    })
}
