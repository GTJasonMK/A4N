use a4n_domain::shooting_script::EditStoredChapterShootingScriptInput;
use sea_orm::TransactionTrait;

use super::{
    codec::stored_visual_plan_identity,
    contracts::{
        EditChapterShootingScriptOutcome, ShootingScriptDbError, UpdateChapterShootingScriptInput,
    },
    mutations::update_chapter_shooting_script,
    queries::chapter_shooting_script_by_id,
};

pub async fn edit_chapter_shooting_script_and_invalidate(
    db: &sea_orm::DatabaseConnection,
    input: EditStoredChapterShootingScriptInput,
) -> Result<EditChapterShootingScriptOutcome, ShootingScriptDbError> {
    let tx = db.begin().await?;
    let current = chapter_shooting_script_by_id(&tx, input.record_id).await?;
    if current.record.revision != input.expected_revision {
        return Err(ShootingScriptDbError::RevisionConflict {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
        });
    }
    let visual_plan = stored_visual_plan_identity(&current.record)?.ok_or_else(|| {
        ShootingScriptDbError::InvalidState(
            "历史拍摄剧本未绑定视觉方案，不能继续编辑；请重新生成绑定当前视觉方案的拍摄剧本"
                .to_string(),
        )
    })?;
    let shooting_script = update_chapter_shooting_script(
        &tx,
        UpdateChapterShootingScriptInput {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
            input_fingerprint_sha256: current.record.input_fingerprint_sha256.clone(),
            visual_plan,
            script: input.script,
            workflow_run_id: None,
        },
    )
    .await?;
    if shooting_script.record.revision == current.record.revision {
        tx.commit().await?;
        return Ok(EditChapterShootingScriptOutcome {
            shooting_script,
            invalidated_project_ids: Vec::new(),
        });
    }
    let invalidated_project_ids = crate::project::projects_using_source_snapshot(
        &tx,
        current.record.root_project_id,
        current.record.source_snapshot_id,
    )
    .await?;
    tx.commit().await?;
    Ok(EditChapterShootingScriptOutcome {
        shooting_script,
        invalidated_project_ids,
    })
}
